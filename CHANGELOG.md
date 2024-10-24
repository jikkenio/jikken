# Next (Version determined when release is cut)

# 0.8.1

# New Features

- Tests can now provide response time constraints to indicate a test failure.
- New `bypass-cert-verification` configuration option to bypass ssl cert verification.

# Changes

- We've migrated to Hyper 1.0 and to Rustls.

# 0.8.0

Jikken now supports secrets, body schemas, and data generation to allow complex schema validation and fuzz testing.
This release opens up support for new types of testing, with more advanced features for data validation and generation.
As always, check the [docs](https://www.jikken.io/docs/) for more details.

# New Features

- Variables can now generate data based on provided constraints for improved fuzz testing
- Added support for variable embedding in `setup`, `clean-up`, and multi-stage response bodies
- Added support for secrets as a secure alternative to global variables
- New `bodySchema` field, which allows more descriptive response validation
- New `platformId` field, to assist in tracking test versions over time (as part of our SaaS platform offering)
- Body comparison now supports strict and non-strict modes
- Improved error messaging around invalid JSON bodies
- OpenApi test generation now supports ref links
- OpenApi test generation can now produce variable definitions for generating test data
- Stages in multi-stage tests can now have names via the `name` field
- New `NOW` and `NOW_UTC` built-in global variables, which contain the current timestamp in local and UTC time
- New `format` command which auto-formats your test files
- New `validate` command for basic test file validation, which provides a mechanism to generate Platform IDs

# Changes

- Displays an error and skips test if test definition contains unknown fields
- Displays a warning when user specified config file is not found
- Some example test API URIs have changed, and the associated tests have been updated
- Split definition for iterative array variables to use a new `valueSet` keyword
- VSCode extension now recognizes the `project` and `valueSet` keywords
- VSCode extension has better support for embedded variables
- Telemetry now includes test stage names
- Telemetry now receives skipped tests information
- Telemetry now receives projects and environments on session create to support alert filtering
- Telemetry now leverages the `platformId` field for tracking test versions over time
- Telemetry no longer sends global variables attached to test data, but instead sends them as part of the session
- Telemetry no longer uses the ID field, which is now reserved for referencing tests as requires
- Test iterations now count as distinct test runs for telemetry data
- Updated Example tests to better showcase tag usage
- Added test execution runtimes to CLI output
- Added restrictions for variable names, which must now contain only alphanumeric characters, hyphens, and underscores

# Bug Fixes

- CLI output now properly flushes prior to test completion (for long-running tests)
- Each run of a test iteration now correctly counts as a distinct test
- Test Definitions no longer include global variables in telemetry, which fixes an issue that may result in inconsistent test ids

# 0.7.2

# New Features:

- Cookie support now also respects `Secure` mode.

# Bugfixes:

- Fixed compare endpoints not properly comparing bodies.

# 0.7.1

# New Features:

- State Variables (extracted from responses) are now able to be embedded into Post Bodies and URLs for subsequent stage/test runs.
- Basic Cookie support. Cookies returned from API endpoints are now added to state and treated as Strict domain/path cookies which pass to future stages.

# 0.7.0

  This release marks a big milestone for Jikken. We've added support for JUnit output and the ability to generate
  tests based on OpenAPI Specs. While these functionalities are working, we plan to incrementally improve them
  over time based on feedback. We've also released a Jikken extension for VSCode to help with writing tests.

# New Features:

- Added a CLI argument which outputs test execution details in the common JUnit format.
- Added support for the `new` command which generates JKT files based on OpenAPI Spec files.
- Added a new command `jk list` which provides a helpful printout for discovered test files.
- Created a VSCode extension for our JKT definition files.
- Added support for variable embedding inside Request bodies.
- Added support for a `.jikkenignore` which lets you specify test files to ignore when executing the test runner.
- Added a `disable` field to the test definition format which allows you to skip tests which would otherwise be included.
- Added a `delay` field to the test definition format which allows you to pause after a test stage before continuing execution.
- Added a `description` field to the test definition format.

# Changes:

- Added local build scripts to support MacOS cross-compiling for Apple Silicon vs x86
- Auth headers are now automatically redacted when enabling test execution telemetry.
  Previously if you used variables for the auth header they were already being restricted but now even if you hardcode
  a secret/key/token in the header it will also be redacted upon telemetry transmission. We still recommend you use
  variables instead of hardcoding credentials in test files.
- Added a badge which tracks our Homebrew release version in the github repo.
- Improved messaging around which tests are executing, being skipped, and why.
- Improved default messaging so that errors/details print after initial test pass/fail line.
- The CLI tool now provides different exit codes based on test success/failure.

# 0.6.2

# New Features:

- Enable project and environment support for test runs. These can be defined via envvar, config, or test definitions.
- Added support for HTTP DELETE test definitions.
- Added CLI argument to provide non-standard config file locations.
- Added new global variable TODAY_UTC to provide today's date based on UTC as opposed to local time.
- Added an optional `name` field for test stages.

# Changes:

- Changed the field name for variable definitions from `dataType` to just `type`.
- The `type` value for Variable definitions now supports other cases (String, string, STRING... etc).
- The windows installer now associates `jkt` files with Jikken.

# Bugfixes:

- Fixed a race condition that would sometimes incorrectly report sessions passing when individual tests failed.
- Fixed a bug with auto-updating version comparisons. Under some conditions the tool would improperly report an older version as being newer.
- Fixed calls which have body data to properly calculate content-type and content-length headers.
- Fixed console colors for legacy windows terminals (ansi-support).

# 0.6.1

# Changes:

- Added unit tests for Config system.

# Bugfixes:

- A println! used for local testing was incorrectly merged into the 0.6.0 release. It has been removed.

# 0.6.0

**BREAKING CHANGE**
In agreement with user feedback, we've decided to make our first breaking change to the test format.
Variables are no longer injected based on the `$var$` convention. They now follow similar
patterns to JavaScript and Bash: `${var}`. Based on discussions and usage we felt it best
to make this change before the number of tests in the wild using variables grows too large.
As always we aim to minimize breaking changes, and since usage is increasing, in the future we'll
likely support backwards compatibility or automated tooling to migrate tests.

We don't foresee any additional breaking changes on the horizon.

# New Features:

- Glob support for matching test files.
- Variables now support loading data from files.
- Our website is now public, which includes a new and improved docs page. Lots of documentation is on the way.

# Changes:

- Adjusted cargo compiler flags for release, greatly reduces release binary size.
- Update cleanup stage definition to use "always" for the always executing request.
- Reduce excessive use of cloning.
- Jikken no longer scans for test files recursively by default. You can now accomplish this via GLOB or with the `-r` CLI argument.
- Update help contents printed to the console.
- We've changed the format for VARIABLE injection. You now must follow the `${var}` pattern instead of the `$var$` pattern.
- Additional unit tests and some code clean-up/refactoring. More to come.
- Added support for HTTP Verbs to be case insensitive (all lower, all upper, or capitalized).

# Bugfixes:

- If a test is not checking response bodies, the test will no longer fail if the response body is not valid JSON.
- If test runs are configured to exit early on failure, the telemetry session completion and console status messages now properly trigger.

# 0.5.0

# New Features:

- Add (optional, opt-in) telemetry support for the Jikken.IO webapp
- Add basic test file validation
- Add support for user-path based configuration file

# Changes:

- Refactored codebase for cleaner more idiomatic rust (module layout)
- Made config files truly optional. If you provide an incorrect config file you will now receive an error but the tests will still run. Previously it would exit early and refuse to run your tests.

# Bugfixes:

- Ignore and Add Parameter definitions now properly work for compare requests

# 0.4.0

# New Features:

- Update Jikken CLI to have command driven execution. Instead of `jk` automatically running tests there are now various commands `jk run`, `jk dryrun` etc.
- Add `jk new` command to create a test file that shows all possible definition options.
- Add `environment` flag to test definition, cli args, config, and environment variables.
- Add `--quiet` flag to disable console output.
- Add `--trace` flag for more detailed output, intended use is for Jikken developers

# Changes:

- Adjusted console output messages around various verbosity levels
- Updated CLI help descriptions

# Bugfixes:

- Fix cleanup stage to properly handle onsuccess and onfailure definitions

# 0.3.0

# New Features:

- Minor adjustments to Example Tests
- Clean-up dry-run console output
- Added support for variable injection into request body definitions
- Added checking for latest version and a self-update command
- Added support for staged test definitions
- Added support for test setup
- Added support for test cleanup
- Added basic example of a multistage test
- Created Windows Installer for releases

# 0.2.0

# New Features:

- Improved test execution with invalid urls. It now properly prints out helpful errors and fails the test
- Consolidated two modes of test runs into a single code path. This simplifies expanding test_runner functionality in the future
- Added dry-run mode. This is a new CLI argument (-d) which prints a description of the steps that will happen without actually calling the apis

# Bugfixes:

- Fix label when printing tests by number. They now print starting at 1 instead of 0.

# 0.1.0

  Initial release of the Jikken CLI tool.

# New Features:

- basic yaml/json test file format to define api tests
- configuration file to support `continue on failure` mode and global variables
- environment variables overriding config file and global variable definition
- tests can be tagged and execution can be invoked to limit based on provided tags
- http requests (GET, POST, PUT, PATCH)
- define headers and body data
- compare result body to pre-defined
- compare status code to pre-defined
- compare two http endpoint results against each other (body and status)
- variable embeddings into tests
- variable extraction from test responses to be embedded into later run tests
- test dependencies (to force order of running tests)
- ignore/prune parts of a json response when comparing body responses
- variable types include `int`, `string`, `date`, and `datetime`
- variables can define sequences for test iterations to cycle through
- date variables support modifiers of adding/subtracting days, weeks, or months
