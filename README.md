# PlayMaster

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


## Variables

PlayMaster supports two sources of variables for your feature tests:

- **Global vars files**: Any `*.vars.yaml` inside `feature_test/`
- **Local vars**: A `vars:` section at the top of each `*.test.yaml`

These variables are simple flat key/value strings and can be referenced in test steps using the `${...}` syntax.

### 1) Global vars files (`*.vars.yaml`)

- Location: place them under `feature_test/`
- File name format: `<Name>.vars.yaml`
- At codegen time (`playmaster gen`), a Dart class is generated for each file in `integration_test/generated/vars.dart`.
  - Class name = `<Name>` converted to PascalCase
  - Each key becomes a `static const` string on that class

Example file `feature_test/common.vars.yaml` (see sample at `samples/flutter_sample_app/feature_test/common.vars.yaml`):

```yaml
validEmail: "qa@test.com"
```

This produces a Dart class similar to:

```dart
// in integration_test/generated/vars.dart
class Common {
  static const validEmail = 'qa@test.com';
}
```

You can then reference it in tests as `${Common.validEmail}`.

### 2) Local vars in a test file

Define a `vars:` mapping at the top of your `*.test.yaml`. Keys and values must be strings.

Example file `feature_test/ftue.test.yaml` (excerpt; see sample at `samples/flutter_sample_app/feature_test/ftue.test.yaml`):

```yaml
name: First Time User Experience
description: >
  Covers the FTUE flow, which is just the login for now

vars:
  validPassword: "password123"
  invalidPassword: "wrongpassword"

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
          value: "${Common.validEmail}"   # from common.vars.yaml
      - type:
          by:
            placeholder: "Password"
          value: "${validPassword}"   # from this file
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

### Interpolation rules

- Use `${Common.key}` to read from a global vars file named `common.vars.yaml` (class `Common`).
- Use `${localKey}` to read from the local `vars:` block in the same `.test.yaml`.
- Vars are plain strings (no nested objects, arrays, or expressions).

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

## Roadmap / TODO

The following features are planned but not yet implemented:

### Remote Test Execution
- **Remote Device Support**: Full implementation of remote test execution on devices (e.g., GameOS devices)
- **Remote Connection Management**: Automated SSH/network connection handling for remote targets
- **Device Discovery**: Automatic discovery of test devices on LAN

### Test Orchestration
- **Multi-Resolution Testing**: Run test suites across multiple screen resolutions automatically
- **Parallel Test Execution**: Run tests concurrently across multiple devices/resolutions
- **LAN Test Orchestration**: Coordinate test execution across multiple machines on local network
- **Cloud Orchestration** (Future): Cloud-based test coordination and scheduling

### Visual Testing & Reporting
- **Screenshot Comparison**: Pixel-perfect screenshot matching for design validation
- **Multi-Resolution Screenshots**: Capture and compare screenshots across different resolutions
- **Golden File Management**: Tools for managing and updating golden screenshot files
- **Test Reports**: Generate comprehensive HTML/JSON test reports with:
  - Test execution results
  - Screenshot diffs and comparisons
  - Performance metrics
  - Historical trend analysis

### Developer Experience
- **Service Management Hooks**: Pre-test hooks for ensuring required services (e.g., playserve) are running
- **Dependency Targeting**: Specify and validate specific versions of dependencies for test runs
- **Better Error Messages**: Enhanced error reporting and debugging information
- **Test Replay/Debug Mode**: Step-through debugging for failed tests

### Framework Support
- **Additional Framework Support**: Extend beyond Flutter (e.g., React Native, native mobile apps)
- **Custom Action Plugins**: Extensible action system for framework-specific test steps

### Configuration
- **Environment Profiles**: Named configuration profiles for different test environments
- **Test Tagging**: Tag tests for selective execution (smoke, regression, etc.)
- **Retry Logic**: Configurable retry strategies for flaky tests

Contributions in any of these areas are welcome! See the Contributing section below for guidelines.

## Contributing

Contributions are welcome! Please ensure:

- Code follows Rust best practices
- Tests pass before submitting PRs
- Documentation is updated for new features

## License

[Apache-2.0 license](LICENSE)

## Support

For issues, questions, or contributions, please open an issue in the repository.
