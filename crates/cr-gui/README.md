# CR GUI - Code Review GUI Application

A graphical user interface for the CR (Code Review) semantic service, built with Rust and egui.

## Features

- **Code Editor**: Syntax-highlighted code editor with file loading/saving capabilities
- **Rule Editor**: Create and edit custom security and code quality rules
- **Results Panel**: View analysis results with severity indicators and filtering
- **Settings Panel**: Configure analysis preferences and appearance
- **Menu Bar**: Access all application features through an intuitive menu system
- **Status Bar**: Monitor analysis progress and system information

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Cargo

### Building and Running

1. Navigate to the cr-gui directory:
   ```bash
   cd crates/cr-gui
   ```

2. Build and run the application:
   ```bash
   cargo run
   ```

### Usage

1. **Loading Code**: 
   - Use File → Open to load a source code file
   - The code will appear in the main editor with syntax highlighting

2. **Creating Rules**:
   - Use the Rule Editor panel to create custom analysis rules
   - Rules support pattern matching and security vulnerability detection

3. **Running Analysis**:
   - Use Tools → Analyze Code to run analysis on the loaded file
   - Results will appear in the Results Panel

4. **Viewing Results**:
   - Click on findings in the Results Panel to see details
   - Filter results by severity or rule type

5. **Settings**:
   - Access View → Settings to configure the application
   - Customize language preferences, editor settings, and appearance

## Architecture

The GUI application is structured as follows:

- `main.rs`: Application entry point and window setup
- `app.rs`: Main application state and UI coordination
- `components/`: Individual UI components
  - `code_editor.rs`: Code editing functionality
  - `rule_editor.rs`: Rule creation and editing
  - `results_panel.rs`: Analysis results display
  - `menu_bar.rs`: Application menu system
  - `settings_panel.rs`: Configuration interface
  - `status_bar.rs`: Status and progress display
- `utils/`: Utility modules for file operations, syntax highlighting, etc.

## Dependencies

- `egui`: Immediate mode GUI framework
- `eframe`: Native application framework for egui
- `rfd`: Native file dialogs
- `chrono`: Date and time handling
- `webbrowser`: Opening URLs in default browser
- `sys-info`: System information

## Testing

Test files are provided in the root directory:
- `test_code.js`: Sample JavaScript file with security vulnerabilities
- `test_rule.yaml`: Sample rule definitions

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is part of the CR semantic service and follows the same license terms.
