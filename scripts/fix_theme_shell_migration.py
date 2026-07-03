#!/usr/bin/env python3
"""Fix broken theme_shell migration in magic-iced examples."""

from __future__ import annotations

import re
from pathlib import Path

EXAMPLES_DIR = Path(__file__).resolve().parents[1] / "crates/magic-iced/examples"
SKIP = {"animated_theme_toggler.rs", "_shared"}


def fix_tick_corruption(content: str) -> str:
    """Fix `(now) = message {` corruption inside Tick handlers."""
    pattern = re.compile(
        r"(Message::Tick\(now\) => \{\s*)\(now\) = message \{\s*"
        r"(self\.theme_shell\.tick\(now\);\s*)",
        re.MULTILINE,
    )
    content = pattern.sub(r"\1\2", content)

    # Remove stray closing braces left by corruption: `}\n            }\n        }` before Task::none
    content = re.sub(
        r"(Message::Tick\(now\) => \{[^}]+)\n\s*\}\n\s*\}\n(\s*\}\n\s*Task::none)",
        r"\1\n            }\n\2",
        content,
        flags=re.DOTALL,
    )
    return content


def fix_themed_shell_args(content: str) -> str:
    """Add missing theme_shell + ToggleTheme args to themed_shell calls."""
    if "themed_shell" not in content or "self.theme_shell" not in content:
        return content

    lines = content.splitlines(keepends=True)
    out: list[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if "_shared::themed_shell(" not in line:
            out.append(line)
            i += 1
            continue

        # Collect themed_shell call block
        block_start = i
        depth = line.count("(") - line.count(")")
        block = [line]
        i += 1
        while i < len(lines) and depth > 0:
            block.append(lines[i])
            depth += lines[i].count("(") - lines[i].count(")")
            i += 1
        block_text = "".join(block)

        if "&self.theme_shell" in block_text:
            out.extend(block)
            continue

        # Insert args before final closing paren of themed_shell
        joined = "".join(block)
        # Find last `)` that closes themed_shell
        insert = ", &self.theme_shell, Message::ToggleTheme"
        # Replace trailing `)\n` before function end - handle multiline
        if joined.rstrip().endswith(")"):
            # Insert before the last closing paren
            idx = joined.rfind(")")
            fixed = joined[:idx] + insert + joined[idx:]
            out.append(fixed)
        else:
            out.extend(block)
    return "".join(out)


def has_theme_shell_field(content: str) -> bool:
    return bool(re.search(r"theme_shell:\s*_shared::ThemeShell", content))


def uses_theme_shell(content: str) -> bool:
    return "self.theme_shell" in content or "theme_shell.tick" in content


def add_theme_shell_field(content: str) -> str:
    if not uses_theme_shell(content) or has_theme_shell_field(content):
        return content

    # Insert field after struct opening line
    match = re.search(r"(struct \w+ \{)\n", content)
    if not match:
        return content
    insert_at = match.end()
    return (
        content[:insert_at]
        + "    theme_shell: _shared::ThemeShell,\n"
        + content[insert_at:]
    )


def add_theme_shell_init(content: str) -> str:
    if not uses_theme_shell(content):
        return content
    if "theme_shell: _shared::ThemeShell::new()" in content:
        return content

    # Self { ... } in new()
    def repl(m: re.Match[str]) -> str:
        body = m.group(1)
        if "theme_shell:" in body:
            return m.group(0)
        return f"Self {{\n                theme_shell: _shared::ThemeShell::new(),\n{body[1:]}"

    content = re.sub(r"Self \{\n(\s+)", repl, content, count=1)
    return content


def ensure_instant_import(content: str) -> str:
    if "Instant" not in content:
        return content
    if re.search(r"use iced::time::.*Instant", content):
        return content
    if "use iced::time::Instant;" in content:
        return content

    # Add after mod _shared;
    if "mod _shared;" in content:
        return content.replace(
            "mod _shared;\n",
            "mod _shared;\n\nuse iced::time::Instant;\n",
            1,
        )
    return content


def ensure_toggle_handlers(content: str) -> str:
    if not uses_theme_shell(content):
        return content
    if "Message::ToggleTheme" in content and "apply_anchor_and_toggle" in content:
        return content

    # Minimal patch for update() that only handles Lens-like cases
    update_match = re.search(
        r"(fn update\(&mut self, message: Message\) -> Task<Message> \{)\n(\s+)(.+?)(\n\s+Task::none\(\)\n\s+\})",
        content,
        re.DOTALL,
    )
    if not update_match:
        return content

    body = update_match.group(3)
    if "ToggleTheme" in body:
        return content

    toggle_block = """match message {
            Message::ToggleTheme => {
                return _shared::ThemeShell::measure_toggle_anchor()
                    .map(Message::AnchorsMeasured);
            }
            Message::AnchorsMeasured(measurements) => {
                self.theme_shell.apply_anchor_and_toggle(&measurements);
            }
            Message::Tick(now) => {
                self.theme_shell.tick(now);
            }
            other => {
                let _ = other;
            }
        }"""

    # If it's a simple if-let pattern, replace whole update body
    if body.strip().startswith("if let"):
        new_update = (
            update_match.group(1)
            + "\n        "
            + toggle_block
            + update_match.group(4)
        )
        return content[: update_match.start()] + new_update + content[update_match.end() :]

    return content


def ensure_message_variants(content: str) -> str:
    if not uses_theme_shell(content):
        return content

    enum_match = re.search(r"enum Message \{([^}]+)\}", content, re.DOTALL)
    if not enum_match:
        return content

    body = enum_match.group(1)
    additions = []
    if "ToggleTheme" not in body:
        additions.append("    ToggleTheme,")
    if "AnchorsMeasured" not in body:
        additions.append("    AnchorsMeasured(magic_iced::AnchorMeasurements),")
    if "Tick(Instant)" not in body and "subscription" in content:
        additions.append("    Tick(Instant),")

    if not additions:
        return content

    new_body = "\n".join(additions) + body
    return content[: enum_match.start(1)] + new_body + content[enum_match.end(1) :]


def ensure_tick_handler_tick(content: str) -> str:
    """Ensure theme_shell.tick is called in Tick handler."""
    if "self.theme_shell.tick(now)" in content:
        return content

    content = re.sub(
        r"(Message::Tick\(now\) => \{)\n(\s+)(?!self\.theme_shell\.tick)",
        r"\1\n\2self.theme_shell.tick(now);\n\2",
        content,
    )
    return content


def process_file(path: Path) -> bool:
    original = path.read_text()
    content = original
    content = fix_tick_corruption(content)
    content = fix_themed_shell_args(content)
    content = add_theme_shell_field(content)
    content = add_theme_shell_init(content)
    content = ensure_instant_import(content)
    content = ensure_message_variants(content)
    content = ensure_toggle_handlers(content)
    content = ensure_tick_handler_tick(content)

    if content != original:
        path.write_text(content)
        return True
    return False


def main() -> None:
    changed = []
    for path in sorted(EXAMPLES_DIR.glob("*.rs")):
        if path.name in SKIP:
            continue
        if process_file(path):
            changed.append(path.name)

    print(f"Updated {len(changed)} files:")
    for name in changed:
        print(f"  - {name}")


if __name__ == "__main__":
    main()
