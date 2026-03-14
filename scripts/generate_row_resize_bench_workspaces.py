#!/usr/bin/env python3

from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
OUTPUT_ROOT = Path("/tmp/novywave_row_resize_bench")


def selected_variable_block(file_path: Path, scope: str, signal: str, signal_type: str) -> str:
    return "\n".join(
        [
            "[[workspace.selected_variables]]",
            f'unique_id = "{file_path}|{scope}|{signal}"',
            'formatter = "DEFAULT"',
            f'signal_type = "{signal_type}"',
            "row_height = 30",
        ]
    )


def write_workspace(name: str, opened_files: list[Path], expanded_scopes: list[str], selected_scope_id: str, selected_blocks: list[str]) -> None:
    workspace_dir = OUTPUT_ROOT / name
    workspace_dir.mkdir(parents=True, exist_ok=True)

    expanded_directories = [
        "/",
        "/home",
        str(REPO_ROOT.parent),
        str(REPO_ROOT),
        str(REPO_ROOT / "test_files"),
    ]

    contents = "\n".join(
        [
            "# Generated benchmark workspace for row-resize testing",
            "",
            "[app]",
            'version = "1.0.0"',
            "",
            "[ui]",
            'theme = "dark"',
            "toast_dismiss_ms = 10000",
            "",
            "[workspace]",
            "opened_files = [",
            *[f'    "{path}",' for path in opened_files],
            "]",
            'dock_mode = "bottom"',
            "expanded_scopes = [",
            *[f'    "{scope}",' for scope in expanded_scopes],
            "]",
            "load_files_expanded_directories = [",
            *[f'    "{path}",' for path in expanded_directories],
            "]",
            f'selected_scope_id = "{selected_scope_id}"',
            "load_files_scroll_position = 0",
            "",
            "[workspace.docked_bottom_dimensions]",
            "files_and_scopes_panel_width = 400.0",
            "files_and_scopes_panel_height = 480.0",
            "selected_variables_panel_name_column_width = 220.0",
            "selected_variables_panel_value_column_width = 220.0",
            "",
            "[workspace.docked_right_dimensions]",
            "files_and_scopes_panel_width = 400.0",
            "files_and_scopes_panel_height = 300.0",
            "selected_variables_panel_name_column_width = 220.0",
            "selected_variables_panel_value_column_width = 220.0",
            "",
            *selected_blocks,
            "",
            "[workspace.timeline]",
            "cursor_position_ps = 0",
            "visible_range_start_ps = 0",
            "visible_range_end_ps = 1000000000000",
            "zoom_center_ps = 0",
            "tooltip_enabled = true",
            "",
            "[plugins]",
            "schema_version = 1",
            "entries = []",
            "",
        ]
    )

    (workspace_dir / ".novywave").write_text(contents)
    print(workspace_dir)


def main() -> None:
    protocol = REPO_ROOT / "test_files" / "protocol_test.vcd"
    simple_reload = REPO_ROOT / "test_files" / "simple_reload_test.vcd"
    analog = REPO_ROOT / "test_files" / "analog.vcd"
    stress = REPO_ROOT / "test_files" / "stress_test.vcd"

    protocol_rows = [
        ("protocols.i2c", "sda", "Wire"),
        ("protocols.i2c", "scl", "Wire"),
        ("protocols.spi", "mosi", "Wire"),
        ("protocols.spi", "miso", "Wire"),
        ("protocols.spi", "sclk", "Wire"),
        ("protocols.spi", "cs", "Wire"),
        ("protocols.uart", "tx", "Wire"),
        ("protocols.uart", "rx", "Wire"),
        ("protocols.memory", "address", "Wire"),
        ("protocols.memory", "data", "Wire"),
        ("protocols.memory", "write_enable", "Wire"),
        ("protocols.memory", "read_enable", "Wire"),
        ("protocols.axi", "awaddr", "Wire"),
        ("protocols.axi", "wdata", "Wire"),
        ("protocols.axi", "wstrb", "Wire"),
        ("protocols.axi", "awvalid", "Wire"),
        ("protocols.axi", "awready", "Wire"),
    ]
    common_rows = [
        *[(protocol, scope, signal, signal_type) for scope, signal, signal_type in protocol_rows],
        (simple_reload, "simple_tb.s", "A", "Wire"),
        (simple_reload, "simple_tb.s", "C", "Wire"),
        (analog, "top", "analog", "Real"),
    ]
    stress_rows = [
        ("stress_test.rapid_changes", "toggle_fast", "Wire"),
        ("stress_test.rapid_changes", "byte_pattern", "Wire"),
        ("stress_test.rapid_changes", "word_counter", "Wire"),
        ("stress_test.rapid_changes", "random_data", "Wire"),
        ("stress_test.edge_cases", "x_state", "Wire"),
        ("stress_test.edge_cases", "z_state", "Wire"),
        ("stress_test.edge_cases", "unknown_byte", "Wire"),
        ("stress_test.edge_cases", "mixed_states", "Wire"),
        ("stress_test.long_values", "kilobit_bus", "Wire"),
        ("stress_test.long_values", "two_kilobit", "Wire"),
        ("stress_test.strings", "ascii_text", "Wire"),
        ("stress_test.strings", "status_code", "Wire"),
    ]

    write_workspace(
        name="many_rows_valid",
        opened_files=[protocol, simple_reload, analog],
        expanded_scopes=[
            str(protocol),
            f"scope_{protocol}|protocols",
            f"scope_{protocol}|protocols.i2c",
            f"scope_{protocol}|protocols.spi",
            f"scope_{protocol}|protocols.uart",
            f"scope_{protocol}|protocols.memory",
            f"scope_{protocol}|protocols.axi",
            str(simple_reload),
            f"scope_{simple_reload}|simple_tb",
            str(analog),
            f"scope_{analog}|top",
        ],
        selected_scope_id=f"scope_{protocol}|protocols.axi",
        selected_blocks=[
            selected_variable_block(file_path, scope, signal, signal_type)
            for file_path, scope, signal, signal_type in common_rows
        ],
    )

    write_workspace(
        name="stress_rows_valid",
        opened_files=[protocol, simple_reload, analog, stress],
        expanded_scopes=[
            str(protocol),
            f"scope_{protocol}|protocols",
            f"scope_{protocol}|protocols.memory",
            f"scope_{protocol}|protocols.axi",
            str(simple_reload),
            f"scope_{simple_reload}|simple_tb",
            str(analog),
            f"scope_{analog}|top",
            str(stress),
            f"scope_{stress}|stress_test",
            f"scope_{stress}|stress_test.rapid_changes",
            f"scope_{stress}|stress_test.edge_cases",
            f"scope_{stress}|stress_test.long_values",
            f"scope_{stress}|stress_test.strings",
        ],
        selected_scope_id=f"scope_{stress}|stress_test.long_values",
        selected_blocks=[
            selected_variable_block(file_path, scope, signal, signal_type)
            for file_path, scope, signal, signal_type in (
                common_rows
                + [(stress, scope, signal, signal_type) for scope, signal, signal_type in stress_rows]
            )
        ],
    )


if __name__ == "__main__":
    main()
