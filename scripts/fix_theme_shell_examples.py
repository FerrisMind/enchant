#!/usr/bin/env python3
"""Fix broken theme shell migrations."""

from __future__ import annotations

import re
from pathlib import Path

EXAMPLES_DIR = Path(__file__).resolve().parents[1] / "crates/magic-iced/examples"
SKIP = {"animated_theme_toggler.rs"}


def fix_content(content: str) -> str:
    # struct field type
    content = re.sub(
        r"^(\s*)theme_shell: _shared::ThemeShell::new\(\),$",
        r"\1theme_shell: _shared::ThemeShell,",
        content,
        flags=re.M,
    )

    # broken themed_shell / tabs replacements
    content = content.replace("&self.theme_shell.theme()_shell", "&self.theme_shell")

    # spacing(24, &self.theme_shell -> spacing(24), &self.theme_shell
    content = re.sub(
        r"\.spacing\((\d+), &self\.theme_shell, Message::ToggleTheme\)\.align_x\(([^)]+)\)",
        r".spacing(\1).align_x(\2), &self.theme_shell, Message::ToggleTheme",
        content,
    )
    content = re.sub(
        r"\.spacing\((\d+), &self\.theme_shell, Message::ToggleTheme\)",
        r".spacing(\1), &self.theme_shell, Message::ToggleTheme",
        content,
    )

    # column![tabs, _shared::stage(body)].spacing(24), &self.theme_shell, Message::ToggleTheme)
    content = re.sub(
        r"_shared::themed_shell\(([^,]+), (column!\[[^\]]+\]), &self\.theme_shell, Message::ToggleTheme\)\)",
        r"_shared::themed_shell(\1, \2, &self.theme_shell, Message::ToggleTheme)",
        content,
    )

    # tabs with extra args
    content = re.sub(
        r"_shared::tabs\(([^,]+), ([^,]+), ([^,]+), &self\.theme_shell, Message::ToggleTheme\)",
        r"_shared::tabs(\1, \2, \3)",
        content,
    )

    # stage(body, &self.theme_shell, ...)
    content = re.sub(
        r"_shared::stage\(([^,]+), &self\.theme_shell, Message::ToggleTheme\)",
        r"_shared::stage(\1)",
        content,
    )

    # text("...", &self.theme_shell, Message::ToggleTheme)
    content = re.sub(
        r"text\(([^)]+), &self\.theme_shell, Message::ToggleTheme\)",
        r"text(\1)",
        content,
    )

    # duplicate subscription lines
    content = re.sub(
        r"(\.subscription\([^)]+\)\n\s*)\.subscription\([^)]+\)",
        r"\1",
        content,
    )

    # add theme_shell.tick in Tick arms if missing
    if "self.theme_shell.tick" not in content and "Message::Tick" in content:
        content = re.sub(
            r"(Message::Tick\(now\) => \{\n)(\s*)(self\.now = now;)?",
            r"\1\2self.theme_shell.tick(now);\n\2\3",
            content,
            count=1,
        )
        content = re.sub(
            r"(Message::Tick\(now\) => \{\n)(\s*)self\.theme_shell\.tick\(now\);\n\2self\.theme_shell\.tick\(now\);",
            r"\1\2self.theme_shell.tick(now);",
            content,
        )

    # missing use Instant for dock etc
    if "Tick(Instant)" in content and "use iced::time::Instant" not in content:
        if "use iced::time::" in content:
            content = content.replace(
                "use iced::time::{Duration, Instant};",
                "use iced::time::{Duration, Instant};",
            )
        elif "Subscription" in content or "Tick(Instant)" in content:
            content = content.replace(
                "use iced::{Element, Size, Task};",
                "use iced::time::Instant;\nuse iced::{Element, Size, Subscription, Task};",
            )

    return content


def main() -> None:
    for path in sorted(EXAMPLES_DIR.glob("*.rs")):
        if path.name in SKIP:
            continue
        original = path.read_text()
        fixed = fix_content(original)
        if fixed != original:
            path.write_text(fixed)
            print(path.name)


if __name__ == "__main__":
    main()
