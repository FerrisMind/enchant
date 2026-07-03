#!/usr/bin/env python3
"""Add ThemeShell + themed_shell to magic-iced examples."""

from __future__ import annotations

import re
from pathlib import Path

EXAMPLES_DIR = Path(__file__).resolve().parents[1] / "crates/magic-iced/examples"
SKIP = {"animated_theme_toggler.rs"}

THEME_MSG_VARIANTS = """    ToggleTheme,
    AnchorsMeasured(magic_iced::AnchorMeasurements),"""


def ensure_mod_shared(content: str) -> str:
    if "mod _shared;" in content:
        return content
    return "mod _shared;\n\n" + content


def replace_theme_field(content: str) -> str:
    content = re.sub(
        r"theme_shell:\s*_shared::ThemeShell",
        "theme_shell: _shared::ThemeShell",
        content,
    )
    if "theme_shell:" in content:
        return content

    content = re.sub(
        r"theme:\s*MagicTheme::dark\(\)",
        "theme_shell: _shared::ThemeShell::new_dark()",
        content,
    )
    content = re.sub(
        r"theme:\s*Theme::default\(\)",
        "theme_shell: _shared::ThemeShell::new()",
        content,
    )
    content = re.sub(
        r"theme:\s*Theme::from_semantic_theme\([^)]+\)",
        "theme_shell: _shared::ThemeShell::new_dark()",
        content,
        flags=re.DOTALL,
    )
    content = re.sub(r"theme:\s*Theme,", "theme_shell: _shared::ThemeShell,", content)
    return content


def fix_theme_shell_init(content: str) -> str:
    content = re.sub(
        r"theme_shell:\s*_shared::ThemeShell,",
        "theme_shell: _shared::ThemeShell::new(),",
        content,
    )
    return content


def add_message_variants(content: str) -> str:
    if "ToggleTheme" in content and "AnchorsMeasured" in content:
        return content

    def repl(match: re.Match[str]) -> str:
        body = match.group(1)
        if "ToggleTheme" in body:
            return match.group(0)
        return match.group(0) + "\n" + THEME_MSG_VARIANTS

    return re.sub(r"(enum Message \{)", repl, content, count=1)


def inject_update_handlers(content: str) -> str:
    if "apply_anchor_and_toggle" in content:
        return content

    toggle_block = """            Message::ToggleTheme => {
                return _shared::ThemeShell::measure_toggle_anchor()
                    .map(Message::AnchorsMeasured);
            }
            Message::AnchorsMeasured(measurements) => {
                self.theme_shell.apply_anchor_and_toggle(&measurements);
            }
"""

    tick_line = "            self.theme_shell.tick(now);\n"

    # if let Message::Tick(now) = message;
    if re.search(r"if let Message::Tick\(now\) = message", content):
        content = re.sub(
            r"(if let Message::Tick\(now\) = message \{)",
            r"\1\n" + tick_line,
            content,
            count=1,
        )
        # Convert to match if needed - insert toggle before if let
        content = re.sub(
            r"fn update\(&mut self, message: Message\) -> Task<Message> \{\n\s*if let Message::Tick",
            "fn update(&mut self, message: Message) -> Task<Message> {\n        match message {\n"
            + toggle_block
            + "            Message::Tick(now) => {\n",
            content,
            count=1,
        )
        content = content.replace(
            "        match message {\n"
            + toggle_block
            + "            Message::Tick(now) => {\n        if let Message::Tick(now) = message {",
            "        match message {\n" + toggle_block + "            Message::Tick(now) => {",
        )
        # Close match arm and match
        content = re.sub(
            r"(self\.theme_shell\.tick\(now\);\n(?:.*\n)*?)(\s*Task::none\(\)\n\s*\})",
            lambda m: m.group(1) + "            }\n        }\n        Task::none()\n    }",
            content,
            count=1,
        )
        return content

    # match message { ... }
    if re.search(r"fn update\(&mut self, message: Message\) -> Task<Message> \{\s*match message", content):
        return re.sub(
            r"(match message \{)",
            r"\1\n" + toggle_block,
            content,
            count=1,
        )

    # match message with first arm
    if "fn update" in content and "match message" in content:
        if "ToggleTheme" not in content:
            content = re.sub(
                r"(match message \{)",
                r"\1\n" + toggle_block,
                content,
                count=1,
            )
        # add tick to Tick arm
        if "self.theme_shell.tick" not in content:
            content = re.sub(
                r"(Message::Tick\(now\) => \{)",
                r"\1\n" + tick_line,
                content,
                count=1,
            )
        return content

    return content


def replace_shell_calls(content: str) -> str:
    if "_shared::themed_shell(" in content:
        return content

    def repl(match: re.Match[str]) -> str:
        args = match.group(1)
        # already has theme_shell args
        if "theme_shell" in args:
            return match.group(0)
        parts = split_top_level_commas(args)
        if len(parts) == 2:
            title, body = parts
            return (
                f"_shared::themed_shell({title}, {body}, "
                f"&self.theme_shell, Message::ToggleTheme)"
            )
        return match.group(0)

    return re.sub(r"_shared::shell\(([^)]*(?:\([^)]*\)[^)]*)*)\)", repl, content)


def split_top_level_commas(s: str) -> list[str]:
    parts: list[str] = []
    depth = 0
    current: list[str] = []
    for ch in s:
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
        if ch == "," and depth == 0:
            parts.append("".join(current).strip())
            current = []
        else:
            current.append(ch)
    if current:
        parts.append("".join(current).strip())
    return parts


def replace_theme_refs(content: str) -> str:
    content = content.replace("&self.theme", "&self.theme_shell.theme()")
    content = content.replace("self.theme.", "self.theme_shell.theme().")
    return content


def ensure_subscription(content: str, struct_name: str) -> str:
    if ".subscription(" in content:
        if "self.theme_shell.tick" in content or "ThemeShell::tick_subscription" in content:
            return content
        # add tick to existing subscription return - skip complex cases
        return content

    if "Tick(Instant)" not in content:
        content = add_message_variants(content)
        if "Tick(Instant)" not in content:
            content = re.sub(
                r"(enum Message \{[^}]*ToggleTheme,)",
                r"\1\n    Tick(Instant),",
                content,
                count=1,
            )

    if f"fn subscription(&self)" in content:
        return content

    impl_match = re.search(rf"impl {struct_name}", content)
    if not impl_match:
        return content

    subscription_fn = f"""
    fn subscription(&self) -> iced::Subscription<Message> {{
        _shared::ThemeShell::tick_subscription(Message::Tick)
    }}
"""

    # insert before last closing brace of impl - before final fn view or at end
    if "fn view(&self)" in content:
        content = content.replace(
            "    fn view(&self)",
            subscription_fn + "\n    fn view(&self)",
            1,
        )
    else:
        content = content.rstrip() + subscription_fn + "}\n"

    if ".subscription(" not in content:
        content = content.replace(
            ".run()",
            f".subscription({struct_name}::subscription)\n        .run()",
        )
        content = content.replace(
            ".run()\n",
            f".subscription({struct_name}::subscription)\n    .run()\n",
        )

    # ensure update handles Tick
    if "Message::Tick" not in content and "Tick(Instant)" in content:
        content = re.sub(
            r"(match message \{)",
            r"\1\n            Message::Tick(now) => {\n                self.theme_shell.tick(now);\n            }\n",
            content,
            count=1,
        )

    return content


def remove_unused_theme_import(content: str) -> str:
    if "Theme::" not in content and "&Theme" not in content and ": Theme" not in content:
        content = re.sub(r"^use iced_shadcn::Theme;\n", "", content, flags=re.M)
    if "MagicTheme" not in content:
        content = re.sub(r"^use magic_iced::\{([^}]*?)MagicTheme, ?", r"use magic_iced::{\1", content)
        content = re.sub(r", MagicTheme", "", content)
    return content


def struct_name_from_file(content: str) -> str | None:
    m = re.search(r"struct (\w+)", content)
    return m.group(1) if m else None


def process_file(path: Path) -> bool:
    if path.name in SKIP:
        return False

    original = path.read_text()
    content = original
    content = ensure_mod_shared(content)
    content = replace_theme_field(content)
    content = fix_theme_shell_init(content)
    content = add_message_variants(content)
    content = inject_update_handlers(content)
    content = replace_shell_calls(content)
    content = replace_theme_refs(content)

    struct_name = struct_name_from_file(content)
    if struct_name:
        content = ensure_subscription(content, struct_name)

    content = remove_unused_theme_import(content)

    if content != original:
        path.write_text(content)
        return True
    return False


def main() -> None:
    changed = []
    for path in sorted(EXAMPLES_DIR.glob("*.rs")):
        if process_file(path):
            changed.append(path.name)
    print(f"Updated {len(changed)} files:")
    for name in changed:
        print(f"  - {name}")


if __name__ == "__main__":
    main()
