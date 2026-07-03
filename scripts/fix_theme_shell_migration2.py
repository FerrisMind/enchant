#!/usr/bin/env python3
"""Second-pass fixes for theme_shell migration."""

from __future__ import annotations

import re
from pathlib import Path

EXAMPLES_DIR = Path(__file__).resolve().parents[1] / "crates/magic-iced/examples"
SKIP = {"animated_theme_toggler.rs"}


def fix_double_comma_themed_shell(content: str) -> str:
    content = re.sub(
        r"(\.spacing\([^)]+\),)\n\s*,\s*&self\.theme_shell",
        r"\1\n            &self.theme_shell",
        content,
    )
    content = re.sub(
        r"(\.spacing\([^)]+\),)\n\s*,\s*&self\.theme_shell",
        r"\1\n            &self.theme_shell",
        content,
    )
    content = re.sub(
        r"(\.align_x\([^)]+\),)\n\s*\)\n(\s*\})",
        r"\1,\n            &self.theme_shell, Message::ToggleTheme)\n\2",
        content,
    )
    return content


def fix_themed_shell_two_args(content: str) -> str:
    """Fix themed_shell calls that end with `        )` without theme args."""
    pattern = re.compile(
        r"_shared::themed_shell\((.*?)\n\s+\)\n(\s+\})",
        re.DOTALL,
    )

    def repl(m: re.Match[str]) -> str:
        inner = m.group(1)
        if "&self.theme_shell" in inner:
            return m.group(0)
        return (
            f"_shared::themed_shell({inner},\n            &self.theme_shell, Message::ToggleTheme)\n{m.group(2)}"
        )

    return pattern.sub(repl, content)


def fix_theme_temporary_borrow(content: str) -> str:
    if "&self.theme_shell.theme()" not in content:
        return content

    content = content.replace("&self.theme_shell.theme()", "&theme")

    # Insert `let theme = self.theme_shell.theme();` at start of view if not present
    if "let theme = self.theme_shell.theme()" in content:
        return content

    def repl(m: re.Match[str]) -> str:
        return m.group(1) + "\n        let theme = self.theme_shell.theme();\n"

    return re.sub(r"(fn view\(&self\) -> Element<'_, Message> \{)\n", repl, content, count=1)


def fix_confetti_new(content: str) -> str:
    if "ConfettiExample" not in content:
        return content
    return content.replace(
        "Self { state, props, now }",
        "Self {\n                theme_shell: _shared::ThemeShell::new(),\n                state,\n                props,\n                now,\n            }",
    )


def fix_lens(content: str) -> str:
    if "LensExample" not in content:
        return content
    content = content.replace(
        "(Self { state: LensState::new() }, Task::none())",
        "(Self {\n            theme_shell: _shared::ThemeShell::new(),\n            state: LensState::new(),\n        }, Task::none())",
    )
    content = content.replace(
        """            other => {
                let _ = other;
            }""",
        """            Message::Lens(action) => {
                self.state.apply_action(&action);
            }""",
    )
    return content


def fix_missing_tick_handlers(content: str) -> str:
    if "Message::Tick(Instant)" not in content or "self.theme_shell.tick(now)" in content:
        return content

    # Add theme_shell.tick to existing Tick arm
    content = re.sub(
        r"(Message::Tick\(now\) => \{)\n(\s+)(?!self\.theme_shell\.tick)",
        r"\1\n\2self.theme_shell.tick(now);\n\2",
        content,
    )

    # Add Tick arm before closing of match if ToggleTheme exists but no Tick arm
    if "Message::Tick(now)" in content and re.search(
        r"Message::Tick\(now\) =>", content
    ):
        return content

    if "Message::ToggleTheme" not in content:
        return content

    tick_arm = """
            Message::Tick(now) => {
                self.theme_shell.tick(now);
            }"""

    return re.sub(
        r"(match message \{.*?)(\n\s+\}\n\s+Task::none)",
        lambda m: m.group(1) + tick_arm + m.group(2),
        content,
        count=1,
        flags=re.DOTALL,
    )


def fix_enum_formatting(content: str) -> str:
    return content.replace("AnchorMeasurements),}", "AnchorMeasurements),\n}")


def ensure_instant_import(content: str) -> str:
    if "Instant" not in content:
        return content
    if "use iced::time::Instant" in content or "use iced::time::{Duration, Instant}" in content:
        return content
    return content.replace("mod _shared;\n", "mod _shared;\n\nuse iced::time::Instant;\n", 1)


def fix_grid_pattern_update(content: str) -> str:
    if "GridPatternExample" not in content:
        return content
    if "apply_anchor_and_toggle" in content:
        return content
    return content.replace(
        "fn update(&mut self, _message: Message) -> Task<Message> {\n        Task::none()\n    }",
        """fn update(&mut self, message: Message) -> Task<Message> {
        match message {
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
        }
        Task::none()
    }""",
    )


def process_file(path: Path) -> bool:
    original = path.read_text()
    content = original
    content = fix_double_comma_themed_shell(content)
    content = fix_themed_shell_two_args(content)
    content = fix_theme_temporary_borrow(content)
    content = fix_confetti_new(content)
    content = fix_lens(content)
    content = fix_missing_tick_handlers(content)
    content = fix_enum_formatting(content)
    content = ensure_instant_import(content)
    content = fix_grid_pattern_update(content)

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
    print(f"Updated {len(changed)} files")
    for name in changed:
        print(f"  - {name}")


if __name__ == "__main__":
    main()
