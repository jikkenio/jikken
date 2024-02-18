Next (Version determined when release is cut)
=====

Changes:
* Changed the field name for variable definitions from `dataType` to just `type`.
* The `type` value for Variable definitions now supports other cases (String, string, STRING... etc).

0.6.1
=====

Bugfixes:
* A println! used for local testing was incorrectly merged into the 0.6.0 release. It has been removed.

Changes:
* Added unit tests for Config system.

0.6.0
=====

**BREAKING CHANGE**
In agreement with user feedback, we've decided to make our first breaking change to the test format.
Variables are no longer injected based on the `$var$` convention. They now follow similar
patterns to JavaScript and Bash: `${var}`. Based on discussions and usage we felt it best
to make this change before the number of tests in the wild using variables grows too large.
As always we aim to minimize breaking changes, and since usage is increasing, in the future we'll
likely support backwards compatibility or automated tooling to migrate tests.

We don't forsee any additional breaking changes on the horizon. 

Bugfixes:
* If a test is not checking response bodies, the test will no longer fail if the response body is not valid JSON.
* If test runs are configured to exit early on failure, the telemetry session completion and console status messages now properly trigger.

Features:
* Glob support for matching test files.
* Variables now support loading data from files.
* Our website is now public, which includes a new and improved docs page. Lots of documentation is on the way.

Changes:
* Adjusted cargo compiler flags for release, greatly reduces release binary size.
* Update cleanup stage definition to use "always" for the always executing request.
* Reduce excessive use of cloning.
* Jikken no longer scans for test files recursively by default. You can now accomplish this via GLOB or with the `-r` CLI argument.
* Update help contents printed to the console.
* We've changed the format for VARIABLE injection. You now must follow the `${var}` pattern instead of the `$var$` pattern.
* Additional unit tests and some code clean-up/refactoring. More to come.
* Added support for HTTP Verbs to be case insensitive (all lower, all upper, or capitalized).

0.5.0
=====

Bugfixes:
* Ignore and Add Parameter definitions now properly work for compare requests

Features:
* Add (optional, opt-in) telemetry support for the Jikken.IO webapp
* Add basic test file validation
* Add support for user-path based configuration file

Changes:
* Refactored codebase for cleaner more idiomatic rust (module layout)
* Made config files truly optional. If you provide an incorrect config file you will now receive an error but the tests will still run. Previously it would exit early and refuse to run your tests.

0.4.0
=====

Bugfixes:
* Fix cleanup stage to properly handle onsuccess and onfailure definitions

Changes:
* Adjusted console output messages around various verbosity levels
* Updated CLI help descriptions

Features:
* Update Jikken CLI to have command driven execution. Instead of `jk` automatically running tests there are now various commands `jk run`, `jk dryrun` etc.
* Add `jk new` command to create a test file that shows all possible definition options.
* Add `environment` flag to test definition, cli args, config, and environment variables.
* Add `--quiet` flag to disable console output.
* Add `--trace` flag for more detailed output, intended use is for Jikken developers

0.3.0
=====

Features:
* Minor adjustments to Example Tests
* Clean-up dry-run console output
* Added support for variable injection into request body definitions
* Added checking for latest version and a self-update command
* Added support for staged test definitions
* Added support for test setup
* Added support for test cleanup
* Added basic example of a multistage test
* Created Windows Installer for releases

0.2.0
=====

Bugfixes:
* Fix label when printing tests by number. They now print starting at 1 instead of 0.

Features:
* Improved test execution with invalid urls. It now properly prints out helpful errors and fails the test
* Consolidated two modes of test runs into a single code path. This simplifies expanding test_runner functionality in the future
* Added dry-run mode. This is a new CLI argument (-d) which prints a description of the steps that will happen without actually calling the apis

0.1.0
=====
Initial release of the Jikken CLI tool.

Features:

* basic yaml/json test file format to define api tests
* configuration file to support `continue on failure` mode and global variables
* environment variables overriding config file and global variable definition
* tests can be tagged and execution can be invoked to limit based on provided tags
* http requests (GET, POST, PUT, PATCH)
* define headers and body data
* compare result body to pre-defined
* compare status code to pre-defined
* compare two http endpoint results against each other (body and status)
* variable embeddings into tests
* variable extraction from test responses to be embedded into later run tests
* test dependencies (to force order of running tests)
* ignore/prune parts of a json response when comparing body responses
* variable types include `int`, `string`, `date`, and `datetime`
* variables can define sequences for test iterations to cycle through
* date variables support modifiers of adding/subtracting days, weeks, or months
