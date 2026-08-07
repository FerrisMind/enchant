#!/usr/bin/env python3
"""GUI launcher for enchant-iced examples with parity checkboxes."""

from __future__ import annotations

import json
import subprocess
import sys
import threading
import webbrowser
from dataclasses import dataclass
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def examples_dir() -> Path:
    return repo_root() / "crates" / "enchant-iced" / "examples"


def logs_dir() -> Path:
    path = repo_root() / "logs"
    path.mkdir(parents=True, exist_ok=True)
    return path


def state_path() -> Path:
    return logs_dir() / "example-parity.json"


def log_path() -> Path:
    return logs_dir() / "example-parity.log"


def discover_examples() -> list[str]:
    return sorted(
        path.stem
        for path in examples_dir().glob("*.rs")
        if path.name != "_shared.rs"
    )


def utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat()


@dataclass
class ParityState:
    verified: dict[str, bool]
    updated_at: str

    @classmethod
    def load(cls, names: list[str]) -> ParityState:
        path = state_path()
        if not path.exists():
            return cls(verified={name: False for name in names}, updated_at=utc_now())

        try:
            payload = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return cls(verified={name: False for name in names}, updated_at=utc_now())

        verified = {name: False for name in names}
        stored = payload.get("verified", {})
        if isinstance(stored, dict):
            for name in names:
                verified[name] = bool(stored.get(name, False))

        return cls(
            verified=verified,
            updated_at=str(payload.get("updated_at", utc_now())),
        )

    def save(self) -> None:
        payload = {
            "updated_at": utc_now(),
            "verified": self.verified,
            "summary": {
                "total": len(self.verified),
                "ok": sum(1 for ok in self.verified.values() if ok),
                "problems": [
                    name for name, ok in sorted(self.verified.items()) if not ok
                ],
            },
        }
        state_path().write_text(
            json.dumps(payload, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )

    def append_log(self, example: str, ok: bool, note: str = "") -> None:
        status = "OK" if ok else "PROBLEM"
        line = f"{utc_now()}\t{example}\t{status}"
        if note:
            line += f"\t{note}"
        with log_path().open("a", encoding="utf-8") as handle:
            handle.write(line + "\n")

    def append_comment(self, example: str, comment: str) -> bool:
        text = comment.strip()
        if not text:
            return False
        line = f"{utc_now()}\t{example}\tCOMMENT\t{text}"
        with log_path().open("a", encoding="utf-8") as handle:
            handle.write(line + "\n")
        return True

    def write_problems_snapshot(self) -> None:
        problems = [name for name, ok in sorted(self.verified.items()) if not ok]
        ok_count = sum(1 for ok in self.verified.values() if ok)
        total = len(self.verified)
        lines = [
            f"# example parity snapshot — {utc_now()}",
            f"# verified: {ok_count}/{total}",
            "",
        ]
        if problems:
            lines.append("PROBLEMS:")
            lines.extend(f"  - {name}" for name in problems)
        else:
            lines.append("PROBLEMS: none")

        lines.append("")
        lines.append("OK:")
        for name, ok in sorted(self.verified.items()):
            if ok:
                lines.append(f"  - {name}")

        snapshot = logs_dir() / "example-parity-problems.txt"
        snapshot.write_text("\n".join(lines) + "\n", encoding="utf-8")

    def summary(self) -> dict[str, object]:
        ok_count = sum(1 for ok in self.verified.values() if ok)
        return {
            "total": len(self.verified),
            "ok": ok_count,
            "problems": len(self.verified) - ok_count,
        }


def comment_window_url(host: str, port: int, name: str) -> str:
    from urllib.parse import quote

    return f"http://{host}:{port}/comment?name={quote(name)}"


class ExampleRunner:
    def __init__(self) -> None:
        self.lock = threading.Lock()
        self.process: subprocess.Popen[str] | None = None
        self.running: str | None = None

    def run(self, name: str) -> tuple[bool, str]:
        with self.lock:
            if self.process and self.process.poll() is None:
                self.process.terminate()
                try:
                    self.process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    self.process.kill()

            cmd = ["cargo", "run", "-p", "enchant-iced", "--example", name]
            try:
                self.process = subprocess.Popen(
                    cmd,
                    cwd=repo_root(),
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                )
            except OSError as error:
                return False, str(error)

            self.running = name
            return True, f"pid {self.process.pid}"

    def stop(self) -> None:
        with self.lock:
            if self.process and self.process.poll() is None:
                self.process.terminate()
                try:
                    self.process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    self.process.kill()
            self.process = None
            self.running = None

    def status(self) -> dict[str, str | None]:
        with self.lock:
            alive = self.process is not None and self.process.poll() is None
            return {
                "running": self.running if alive else None,
                "pid": str(self.process.pid) if alive and self.process else None,
            }


def build_web_app(
    examples: list[str],
    state: ParityState,
    runner: ExampleRunner,
    host: str = "127.0.0.1",
    port: int = 8765,
) -> type[BaseHTTPRequestHandler]:
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, format: str, *args: object) -> None:
            return

        def _json(self, payload: dict[str, object], code: int = 200) -> None:
            body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
            self.send_response(code)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def _read_json(self) -> dict[str, object]:
            length = int(self.headers.get("Content-Length", "0"))
            if length <= 0:
                return {}
            raw = self.rfile.read(length)
            try:
                payload = json.loads(raw.decode("utf-8"))
            except json.JSONDecodeError:
                return {}
            return payload if isinstance(payload, dict) else {}

        def do_GET(self) -> None:
            path = urlparse(self.path).path
            if path in {"/", "/index.html"}:
                self._html()
                return
            if path == "/comment":
                query = parse_qs(urlparse(self.path).query)
                name = str(query.get("name", [""])[0])
                if name not in state.verified:
                    self.send_error(404)
                    return
                self._comment_html(name)
                return
            if path == "/api/examples":
                rows = [
                    {
                        "name": name,
                        "ok": state.verified.get(name, False),
                    }
                    for name in examples
                ]
                self._json(
                    {
                        "examples": rows,
                        "summary": state.summary(),
                        "runner": runner.status(),
                        "logs": str(logs_dir()),
                    }
                )
                return
            self.send_error(404)

        def do_POST(self) -> None:
            path = urlparse(self.path).path
            payload = self._read_json()

            if path == "/api/toggle":
                name = str(payload.get("name", ""))
                if name not in state.verified:
                    self._json({"error": "unknown example"}, 404)
                    return
                ok = not state.verified[name]
                state.verified[name] = ok
                state.save()
                state.append_log(name, ok, note="toggle")
                state.write_problems_snapshot()
                self._json({"name": name, "ok": ok, "summary": state.summary()})
                return

            if path == "/api/set":
                name = str(payload.get("name", ""))
                if name not in state.verified:
                    self._json({"error": "unknown example"}, 404)
                    return
                ok = bool(payload.get("ok", False))
                state.verified[name] = ok
                state.save()
                state.append_log(name, ok, note="set")
                state.write_problems_snapshot()
                self._json({"name": name, "ok": ok, "summary": state.summary()})
                return

            if path == "/api/run":
                name = str(payload.get("name", ""))
                if name not in state.verified:
                    self._json({"error": "unknown example"}, 404)
                    return
                ok, message = runner.run(name)
                if ok:
                    state.append_log(name, state.verified.get(name, False), note="run")
                    self._json(
                        {
                            "started": name,
                            "message": message,
                            "runner": runner.status(),
                            "comment_url": comment_window_url(host, port, name),
                        }
                    )
                else:
                    self._json({"error": message}, 500)
                return

            if path == "/api/comment":
                name = str(payload.get("name", ""))
                if name not in state.verified:
                    self._json({"error": "unknown example"}, 404)
                    return
                comment = str(payload.get("comment", ""))
                if not state.append_comment(name, comment):
                    self._json({"error": "empty comment"}, 400)
                    return
                self._json({"saved": True, "name": name})
                return

            if path == "/api/stop":
                runner.stop()
                self._json({"stopped": True, "runner": runner.status()})
                return

            self.send_error(404)

        def _html(self) -> None:
            html = WEB_PAGE.encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(html)))
            self.end_headers()
            self.wfile.write(html)

        def _comment_html(self, name: str) -> None:
            html = comment_page_html(name).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(html)))
            self.end_headers()
            self.wfile.write(html)

    return Handler


WEB_PAGE = """<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <title>Magic Iced — Example Launcher</title>
  <style>
    :root { color-scheme: dark; font-family: Inter, system-ui, sans-serif; }
    body { margin: 0; background: #111; color: #eee; }
    .wrap { max-width: 1100px; margin: 0 auto; padding: 20px; }
    h1 { margin: 0 0 8px; font-size: 22px; }
    .sub { color: #aaa; margin-bottom: 16px; }
    .toolbar { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 12px; align-items: center; }
    input, button { background: #1d1d1d; color: #eee; border: 1px solid #333; border-radius: 8px; padding: 8px 12px; }
    button { cursor: pointer; }
    button:hover { background: #262626; }
    .summary { margin-left: auto; color: #9cdcfe; }
    table { width: 100%; border-collapse: collapse; background: #171717; border: 1px solid #2a2a2a; border-radius: 12px; overflow: hidden; }
    th, td { padding: 10px 12px; border-bottom: 1px solid #242424; }
    th { text-align: left; background: #1b1b1b; }
    tr:hover { background: #202020; }
    tr.selected { background: #243041; }
    .ok-col { width: 72px; text-align: center; }
    .status { margin-top: 12px; color: #bdbdbd; min-height: 1.2em; }
    .problem { color: #f48771; }
    .good { color: #89d185; }
    input[type="checkbox"] { width: 18px; height: 18px; }
  </style>
</head>
<body>
  <div class="wrap">
    <h1>Magic Iced — Example Launcher</h1>
    <div class="sub">Галочка справа = компонент совпадает с референсом. Без галочки = есть проблема. При запуске открывается окно комментариев; заметки пишутся в <code>logs/example-parity.log</code>.</div>
    <div class="toolbar">
      <input id="search" placeholder="Поиск..." />
      <button id="runBtn">Запустить</button>
      <button id="stopBtn">Остановить</button>
      <button id="reloadBtn">Обновить</button>
      <div class="summary" id="summary"></div>
    </div>
    <table>
      <thead>
        <tr><th>Пример</th><th class="ok-col">OK</th></tr>
      </thead>
      <tbody id="rows"></tbody>
    </table>
    <div class="status" id="status"></div>
  </div>
  <script>
    let examples = [];
    let selected = null;

    async function api(path, body) {
      const res = await fetch(path, {
        method: body ? "POST" : "GET",
        headers: body ? { "Content-Type": "application/json" } : undefined,
        body: body ? JSON.stringify(body) : undefined,
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || res.statusText);
      return data;
    }

    function renderRows(filter = "") {
      const tbody = document.getElementById("rows");
      tbody.innerHTML = "";
      const q = filter.trim().toLowerCase();
      for (const row of examples) {
        if (q && !row.name.toLowerCase().includes(q)) continue;
        const tr = document.createElement("tr");
        if (selected === row.name) tr.classList.add("selected");
        tr.innerHTML = `<td>${row.name}</td><td class="ok-col"><input type="checkbox" ${row.ok ? "checked" : ""} data-name="${row.name}" /></td>`;
        tr.addEventListener("click", (event) => {
          if (event.target.matches('input[type="checkbox"]')) return;
          selected = row.name;
          renderRows(document.getElementById("search").value);
          setStatus(`Выбран: ${row.name}`);
        });
        tr.querySelector('input[type="checkbox"]').addEventListener("change", async (event) => {
          const name = event.target.dataset.name;
          const ok = event.target.checked;
          const data = await api("/api/set", { name, ok });
          selected = name;
          await refresh(data);
          setStatus(`${name}: ${ok ? "OK" : "нужно исправить"}`, ok ? "good" : "problem");
        });
        tbody.appendChild(tr);
      }
    }

    function setStatus(text, cls = "") {
      const node = document.getElementById("status");
      node.textContent = text;
      node.className = "status " + cls;
    }

    async function refresh(patch) {
      if (patch && patch.summary) {
        document.getElementById("summary").textContent =
          `OK: ${patch.summary.ok}/${patch.summary.total} · проблем: ${patch.summary.problems}`;
      }
      const data = await api("/api/examples");
      examples = data.examples;
      if (!selected && examples.length) selected = examples[0].name;
      document.getElementById("summary").textContent =
        `OK: ${data.summary.ok}/${data.summary.total} · проблем: ${data.summary.problems}`;
      renderRows(document.getElementById("search").value);
      if (data.runner.running) {
        setStatus(`Запущен: ${data.runner.running} (${data.runner.pid})`);
      }
    }

    document.getElementById("search").addEventListener("input", (event) => {
      renderRows(event.target.value);
    });
  function openCommentWindow(name, url) {
    const features = "width=520,height=420,menubar=no,toolbar=no,location=no,status=no";
    const win = window.open(url || `/comment?name=${encodeURIComponent(name)}`, `comment_${name}`, features);
    if (win) win.focus();
  }

    document.getElementById("runBtn").addEventListener("click", async () => {
      if (!selected) return setStatus("Выберите пример", "problem");
      const data = await api("/api/run", { name: selected });
      setStatus(`Запущен: ${data.started} (${data.message})`);
      openCommentWindow(data.started, data.comment_url);
    });
    document.getElementById("stopBtn").addEventListener("click", async () => {
      await api("/api/stop", {});
      setStatus("Остановлено");
    });
    document.getElementById("reloadBtn").addEventListener("click", refresh);
    refresh();
  </script>
</body>
</html>
"""

COMMENT_PAGE = """<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <title>Комментарий — {{EXAMPLE}}</title>
  <style>
    :root { color-scheme: dark; font-family: Inter, system-ui, sans-serif; }
    body { margin: 0; background: #111; color: #eee; }
    .wrap { padding: 16px; display: flex; flex-direction: column; gap: 12px; height: 100vh; box-sizing: border-box; }
    h1 { margin: 0; font-size: 18px; }
    .sub { color: #aaa; font-size: 13px; }
    textarea { flex: 1; resize: none; background: #1d1d1d; color: #eee; border: 1px solid #333; border-radius: 8px; padding: 12px; font: inherit; line-height: 1.4; }
    .toolbar { display: flex; gap: 8px; align-items: center; }
    button { background: #1d1d1d; color: #eee; border: 1px solid #333; border-radius: 8px; padding: 8px 12px; cursor: pointer; }
    button:hover { background: #262626; }
    .status { color: #bdbdbd; min-height: 1.2em; font-size: 13px; }
    .good { color: #89d185; }
    .problem { color: #f48771; }
  </style>
</head>
<body>
  <div class="wrap">
    <div>
      <h1>{{EXAMPLE}}</h1>
      <div class="sub">Комментарий будет записан в <code>logs/example-parity.log</code> для этого примера.</div>
    </div>
    <textarea id="comment" placeholder="Заметки по визуалу, расхождениям с референсом, багам..."></textarea>
    <div class="toolbar">
      <button id="saveBtn">Сохранить в лог</button>
      <div class="status" id="status"></div>
    </div>
  </div>
  <script>
    const example = {{EXAMPLE_JSON}};
    const comment = document.getElementById("comment");
    const status = document.getElementById("status");

    function setStatus(text, cls = "") {
      status.textContent = text;
      status.className = "status " + cls;
    }

    async function saveComment() {
      const text = comment.value.trim();
      if (!text) {
        setStatus("Введите комментарий", "problem");
        return;
      }
      const res = await fetch("/api/comment", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: example, comment: text }),
      });
      const data = await res.json();
      if (!res.ok) {
        setStatus(data.error || res.statusText, "problem");
        return;
      }
      comment.value = "";
      setStatus("Сохранено в лог", "good");
    }

    document.getElementById("saveBtn").addEventListener("click", saveComment);
    comment.addEventListener("keydown", (event) => {
      if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
        event.preventDefault();
        saveComment();
      }
    });
    comment.focus();
  </script>
</body>
</html>
"""


def comment_page_html(name: str) -> str:
    encoded = json.dumps(name, ensure_ascii=False)
    return COMMENT_PAGE.replace("{{EXAMPLE}}", name).replace("{{EXAMPLE_JSON}}", encoded)


def run_web_ui(host: str = "127.0.0.1", port: int = 8765) -> int:
    examples = discover_examples()
    state = ParityState.load(examples)
    state.write_problems_snapshot()
    runner = ExampleRunner()
    handler = build_web_app(examples, state, runner, host=host, port=port)
    server = ThreadingHTTPServer((host, port), handler)
    url = f"http://{host}:{port}/"
    print(f"Example launcher: {url}")
    print(f"Logs: {logs_dir()}")
    threading.Timer(0.4, lambda: webbrowser.open(url)).start()
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nStopping...")
    finally:
        runner.stop()
        state.save()
        state.write_problems_snapshot()
        server.server_close()
    return 0


def run_tk_ui() -> int:
    import tkinter as tk
    from tkinter import messagebox, scrolledtext, ttk

    class CommentWindow(tk.Toplevel):
        def __init__(
            self,
            master: tk.Misc,
            example: str,
            state: ParityState,
            *,
            offset: int = 0,
        ) -> None:
            super().__init__(master)
            self.example = example
            self.state = state
            self.title(f"Комментарий — {example}")
            self.geometry(f"520x420+{40 + offset}+{40 + offset}")
            self.minsize(420, 280)

            frame = ttk.Frame(self, padding=12)
            frame.pack(fill=tk.BOTH, expand=True)

            ttk.Label(
                frame,
                text=f"Пример: {example}",
                font=("", 11, "bold"),
            ).pack(anchor=tk.W)
            ttk.Label(
                frame,
                text="Комментарий будет записан в logs/example-parity.log для этого примера.",
                wraplength=480,
            ).pack(anchor=tk.W, pady=(4, 8))

            self.text = scrolledtext.ScrolledText(frame, wrap=tk.WORD, height=12)
            self.text.pack(fill=tk.BOTH, expand=True, pady=(0, 8))
            self.text.focus_set()

            toolbar = ttk.Frame(frame)
            toolbar.pack(fill=tk.X)

            ttk.Button(toolbar, text="Сохранить в лог", command=self._save).pack(
                side=tk.LEFT
            )
            self.status_var = tk.StringVar()
            ttk.Label(toolbar, textvariable=self.status_var).pack(side=tk.LEFT, padx=(12, 0))

            self.text.bind("<Control-Return>", lambda _event: self._save())
            self.protocol("WM_DELETE_WINDOW", self._on_close)

        def _save(self) -> None:
            comment = self.text.get("1.0", tk.END)
            if not self.state.append_comment(self.example, comment):
                self.status_var.set("Введите комментарий")
                return
            self.text.delete("1.0", tk.END)
            self.status_var.set("Сохранено в лог")

        def _on_close(self) -> None:
            self.destroy()

    class ExampleLauncher(tk.Tk):
        def __init__(self) -> None:
            super().__init__()
            self.title("Magic Iced — Example Launcher")
            self.geometry("920x640")
            self.minsize(720, 480)

            self.examples = discover_examples()
            self.state = ParityState.load(self.examples)
            self.runner = ExampleRunner()
            self.row_vars: dict[str, tk.BooleanVar] = {}
            self.comment_windows: dict[str, CommentWindow] = {}

            self._build_ui()
            self._refresh_summary()
            self.protocol("WM_DELETE_WINDOW", self._on_close)

        def _build_ui(self) -> None:
            root = ttk.Frame(self, padding=12)
            root.pack(fill=tk.BOTH, expand=True)

            ttk.Label(
                root,
                text="Выберите пример, запустите и отметьте галочкой, если компонент совпадает с референсом. При запуске открывается окно комментариев.",
                wraplength=860,
            ).pack(fill=tk.X, pady=(0, 8))

            toolbar = ttk.Frame(root)
            toolbar.pack(fill=tk.X, pady=(0, 8))

            ttk.Label(toolbar, text="Поиск:").pack(side=tk.LEFT)
            self.search_var = tk.StringVar()
            self.search_var.trace_add("write", lambda *_: self._apply_filter())
            ttk.Entry(toolbar, textvariable=self.search_var, width=32).pack(
                side=tk.LEFT, padx=(6, 12)
            )

            ttk.Button(toolbar, text="Запустить", command=self._run_selected).pack(side=tk.LEFT)
            ttk.Button(toolbar, text="Остановить", command=self._stop_running).pack(
                side=tk.LEFT, padx=(6, 0)
            )
            ttk.Button(toolbar, text="Обновить список", command=self._reload_examples).pack(
                side=tk.LEFT, padx=(6, 0)
            )
            ttk.Button(toolbar, text="Открыть logs/", command=self._open_logs_dir).pack(
                side=tk.LEFT, padx=(6, 0)
            )

            self.summary_var = tk.StringVar()
            ttk.Label(toolbar, textvariable=self.summary_var).pack(side=tk.RIGHT)

            columns = ("name", "ok")
            self.tree = ttk.Treeview(
                root,
                columns=columns,
                show="headings",
                selectmode="browse",
                height=24,
            )
            self.tree.heading("name", text="Пример")
            self.tree.heading("ok", text="OK")
            self.tree.column("name", width=700, anchor=tk.W)
            self.tree.column("ok", width=80, anchor=tk.CENTER)
            self.tree.pack(fill=tk.BOTH, expand=True)
            self.tree.bind("<Double-1>", lambda _event: self._run_selected())
            self.tree.bind("<space>", self._toggle_selected_checkbox)
            self.tree.bind("<Button-1>", self._on_tree_click)

            self.status_var = tk.StringVar(value="Готово.")
            ttk.Label(root, textvariable=self.status_var).pack(fill=tk.X, pady=(8, 0))

            self._populate_rows()

        def _populate_rows(self) -> None:
            self.tree.delete(*self.tree.get_children())
            self.row_vars.clear()
            query = self.search_var.get().strip().lower()
            for name in self.examples:
                if query and query not in name.lower():
                    continue
                ok = self.state.verified.get(name, False)
                var = tk.BooleanVar(value=ok)
                self.row_vars[name] = var
                self.tree.insert("", tk.END, iid=name, values=(name, "☑" if ok else "☐"))

        def _apply_filter(self) -> None:
            self._populate_rows()
            self._refresh_summary()

        def _reload_examples(self) -> None:
            self.examples = discover_examples()
            self.state = ParityState.load(self.examples)
            self._populate_rows()
            self._refresh_summary()
            self.status_var.set(f"Найдено примеров: {len(self.examples)}")

        def _refresh_summary(self) -> None:
            summary = self.state.summary()
            self.summary_var.set(
                f"OK: {summary['ok']}/{summary['total']} · проблем: {summary['problems']}"
            )

        def _selected_name(self) -> str | None:
            selection = self.tree.selection()
            return str(selection[0]) if selection else None

        def _on_tree_click(self, event: tk.Event[tk.Misc]) -> str | None:
            if self.tree.identify_region(event.x, event.y) != "cell":
                return None
            if self.tree.identify_column(event.x) != "#2":
                return None
            row_id = self.tree.identify_row(event.y)
            if row_id:
                self._toggle_checkbox(row_id)
            return "break"

        def _toggle_selected_checkbox(self, _event: tk.Event[tk.Misc]) -> None:
            name = self._selected_name()
            if name:
                self._toggle_checkbox(name)

        def _toggle_checkbox(self, name: str) -> None:
            var = self.row_vars.get(name)
            if var is None:
                return
            var.set(not var.get())
            ok = var.get()
            self.state.verified[name] = ok
            self.state.save()
            self.state.append_log(name, ok, note="toggle")
            self.state.write_problems_snapshot()
            self.tree.set(name, "ok", "☑" if ok else "☐")
            self._refresh_summary()
            self.status_var.set(f"{name}: {'OK' if ok else 'нужно исправить'}")

        def _open_comment_window(self, name: str) -> None:
            existing = self.comment_windows.get(name)
            if existing and existing.winfo_exists():
                existing.deiconify()
                existing.lift()
                existing.focus_force()
                existing.text.focus_set()
                return

            offset = 24 * len(self.comment_windows)
            window = CommentWindow(self, name, self.state, offset=offset)
            self.comment_windows[name] = window

            def _forget(_event: tk.Event[tk.Misc] | None = None) -> None:
                if self.comment_windows.get(name) is window:
                    self.comment_windows.pop(name, None)

            window.bind("<Destroy>", _forget)

        def _run_selected(self) -> None:
            name = self._selected_name()
            if not name:
                messagebox.showinfo("Запуск", "Выберите пример в списке.")
                return
            ok, message = self.runner.run(name)
            if ok:
                self.state.append_log(name, self.state.verified.get(name, False), note="run")
                self._open_comment_window(name)
                self.status_var.set(f"Запущен: {name} ({message})")
            else:
                messagebox.showerror("Ошибка запуска", message)

        def _stop_running(self) -> None:
            self.runner.stop()
            self.status_var.set("Пример остановлен.")

        def _open_logs_dir(self) -> None:
            path = logs_dir()
            try:
                if sys.platform.startswith("linux"):
                    subprocess.Popen(["xdg-open", str(path)])
                elif sys.platform == "darwin":
                    subprocess.Popen(["open", str(path)])
                else:
                    subprocess.Popen(["explorer", str(path)])
            except OSError as error:
                messagebox.showinfo("logs/", f"{path}\n\n{error}")

        def _on_close(self) -> None:
            for window in list(self.comment_windows.values()):
                if window.winfo_exists():
                    window.destroy()
            self.comment_windows.clear()
            self.runner.stop()
            self.state.save()
            self.state.write_problems_snapshot()
            self.destroy()

    app = ExampleLauncher()
    app.mainloop()
    return 0


def main() -> int:
    if not examples_dir().is_dir():
        print(f"Examples directory not found: {examples_dir()}", file=sys.stderr)
        return 1

    use_web = "--web" in sys.argv
    if not use_web:
        try:
            import tkinter  # noqa: F401
        except ModuleNotFoundError:
            print("tkinter не найден — открываю web UI в браузере.")
            use_web = True

    if use_web:
        port = 8765
        for arg in sys.argv:
            if arg.startswith("--port="):
                port = int(arg.split("=", 1)[1])
        return run_web_ui(port=port)

    return run_tk_ui()


if __name__ == "__main__":
    raise SystemExit(main())
