# Simple Test Controller

A command-line tool for automating integration tests through declarative YAML configuration. Define your test flows in human-readable YAML files and execute them across local or remote environments. Currently supports Flutter with an extensible architecture for additional frameworks.

## Features

- **Declarative Test Definition**: Write integration tests in YAML with intuitive step-by-step actions
- **Code Generation**: Automatically generate Dart integration test files from YAML definitions
- **Flexible Execution**: Run tests locally or remotely with environment-specific configuration
- **Lifecycle Hooks**: Define custom hooks for system verification, preparation, and test lifecycle events
- **Dependency Management**: Verify system dependencies before test execution
- **JSON Schema Support**: Full schema validation for configuration and test files

## Prerequisites

- Rust (2024 edition or later)
- Cargo build tool
- Project-specific dependencies (e.g., Flutter SDK 3.29.2+ for Flutter projects)

## Installation

Clone the repository and build the project:

```bash
git clone <repository-url>
cd playmaster
cargo build --release
```

The compiled binary will be available at `./target/release/playmaster`.

## Usage

### Commands

The tool provides three main commands:

#### 1. Generate Tests from YAML

```bash
playmaster gen
```

Generates integration test files from YAML feature test definitions in the `feature_test/` directory. The output format depends on your `project_type` configuration (e.g., Dart for Flutter projects).

#### 2. Generate JSON Schemas

```bash
playmaster schema
```

Generates JSON schema files for configuration and feature test validation. Schemas are output to `src/schemas/generated/`:
- `config.json` - Main configuration schema
- `feature_test_schema.json` - Feature test definition schema

#### 3. Run Tests

```bash
# Run tests locally (default)
playmaster run

# Run tests in local mode (explicit)
playmaster run --mode local

# Run tests in remote mode
playmaster run --mode remote
```

Executes the integration tests based on your configuration and feature test definitions.

## Configuration

### Main Configuration File

Create a `playmaster.yaml` in your project root directory:

```yaml
project_type: flutter

dependencies:
  - name: flutter
    min_version: "3.29.2"
    version_command: flutter --version | head -n 1 | awk '{print $2}'

hooks:
  - name: Custom Hook
    hook_type: prepare_system
    command: "echo"
    args: ["Running preparation hook"]
    async: true
    env:
      TEST_ENV: 1
```

**Configuration Schema**: See [config.json](src/schemas/generated/config.json) for the complete JSON schema.

#### Hook Types

Hooks execute at different lifecycle stages:

- `connect` - Establish connections to remote hosts
- `verify_system` - Verify system prerequisites
- `prepare_system` - Prepare the system before tests
- `before_all` - Run before all tests
- `before_test` - Run before each individual test
- `after_test` - Run after each individual test
- `after_all` - Run after all tests complete

### Feature Test Definition

Create YAML test files in the `feature_test/` directory:

```yaml
name: First Time User Experience
description: >
  Covers the FTUE flow, which is just the login for now

tests:
  - name: Successful Login
    description: Enters the correct username and password and logs in
    steps:
      - wait_for:
          text: "Login"
      - tap:
          placeholder: "Email"
      - type:
          by:
            placeholder: "Email"
          value: "qa@test.com"
      - type:
          by:
            placeholder: "Password"
          value: "password123"
      - tap:
          text: "Sign In"
      - wait_for:
          progress: linear
      - wait_for:
          text: "Welcome"
      - match:
          screenshot: "screenshot_welcome"
```

**Feature Test Schema**: See [feature_test_schema.json](src/schemas/generated/feature_test_schema.json) for the complete JSON schema.

#### Available Test Steps

- **wait_for**
  - `text: "string"` - Wait for text to appear
  - `delay: milliseconds` - Wait for a specific duration
  - `progress: linear|radial` - Wait for progress indicator

- **tap**
  - `text: "string"` - Tap element by text
  - `placeholder: "string"` - Tap element by placeholder text

- **type**
  - `by: { text: "string" }` - Type in element found by text
  - `by: { placeholder: "string" }` - Type in element found by placeholder
  - `value: "string"` - Value to type

- **match**
  - `text: "string"` - Assert text exists
  - `screenshot: "name"` - Compare screenshot against golden file

## Project Structure

```
playmaster/
├── src/
│   ├── code_gen/          # Test code generation from YAML
│   ├── code_run/          # Test execution logic
│   ├── hooks/             # Lifecycle hook implementations
│   ├── linux/             # Linux-specific utilities
│   ├── models/            # Data models (config, args, feature tests)
│   ├── schemas/           # JSON schema generation
│   │   └── generated/     # Generated JSON schemas
│   └── utils/             # Utility functions
├── samples/               # Example applications for different project types
│   └── flutter_sample_app/    # Example Flutter application
│       ├── feature_test/      # Example feature test definitions
│       └── playmaster.yaml  # Sample configuration
├── Cargo.toml             # Rust dependencies
└── README.md              # This file
```

## Example Workflow

1. **Set up your project structure**:
   ```
   your_project/
   ├── playmaster.yaml
   └── feature_test/
       └── login.test.yaml
   ```

2. **Define your configuration** (`playmaster.yaml`)

3. **Write feature tests** in `feature_test/*.test.yaml`

4. **Generate schemas** (optional, for IDE autocomplete):
   ```bash
   playmaster schema
   ```

5. **Generate Dart test files**:
   ```bash
   playmaster gen
   ```

6. **Run the tests**:
   ```bash
   playmaster run --mode local
   ```

## Schema Validation

For the best development experience, configure your IDE to use the generated JSON schemas for validation and autocomplete:

### VS Code

Add to your `.vscode/settings.json`:

```json
{
  "yaml.schemas": {
    "src/schemas/generated/config.json": "playmaster.yaml",
    "src/schemas/generated/feature_test_schema.json": "feature_test/*.test.yaml"
  }
}
```

### IntelliJ/WebStorm

1. Go to Settings → Languages & Frameworks → Schemas and DTDs → JSON Schema Mappings
2. Add schema mappings pointing to the generated JSON schemas

## Contributing

Contributions are welcome! Please ensure:

- Code follows Rust best practices
- Tests pass before submitting PRs
- Documentation is updated for new features

## License

[Add your license information here]

## Support

For issues, questions, or contributions, please open an issue in the repository.
