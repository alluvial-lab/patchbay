---
source_handle: alloy-cli
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/AlloyTools/org.alloytools.alloy/v6.2.0/org.alloytools.alloy.cli/src/main/java/org/alloytools/alloy/cli/CLI.java
provenance: source-direct
---

# Attestation: Alloy 6.2.0 CLI source

## Paraphrased summary

The Alloy 6.2.0 CLI source defines an `exec` command for executing Alloy programs, options controlling command selection and output format, parsing and solving behavior, result traces, and generated output files.

## Key passages

{1} The `ExecOptions` interface is annotated with `@Arguments(arg = "path")` and described as executing an Alloy program. The description says execution creates a directory named from the source file stem; solutions are found in that directory; the directory also contains `receipt.json`. Source-internal anchor: `ExecOptions` declaration.

{2} `ExecOptions.command()` is described as selecting the command to run. If no command is specified, the default command will run; the command may use wildcards to run multiple commands; if the command is an integer, it runs the command with that index. Source-internal anchor: `command()` option description.

{3} `ExecOptions.type(OutputType deflt)` is described as selecting output type `none`, `text`, `table`, `json`, or `xml`. Source-internal anchor: `type()` option description.

{4} `ExecOptions.output()` is described as specifying where output goes. The default is a directory with the source-file stem. If the value is `-`, all calculated solutions or transformed files are sent to the console. Source-internal anchor: `output()` option description.

{5} `ExecOptions` also includes `force`, `nooverflow`, `unrolls`, `depth`, `solver`, `quiet`, `evaluator`, and `repeat` options. The `solver` option says solver names can be listed with the `solvers` command and defaults to SAT4J. The `repeat` option says it finds multiple solutions up to the specified number, with `0` for as many as can be found, and default `1`. Source-internal anchor: `ExecOptions` methods.

{6} In `_exec`, the source parses the file with `CompUtil.parseEverything_fromFile`, obtains all commands from the module, determines which commands to run, and invokes `TranslateAlloyToKodkod.execute_commandFromBook` for each selected command. Source-internal anchor: `_exec` implementation.

{7} In `_exec`, if `solution.satisfiable()` is false, the CLI trace writes `UNSAT`; if the command has `expects == 1`, it reports that the command was not satisfied against expectation. If `solution.satisfiable()` is true, the CLI records solution DTOs, generates output, prints a `SAT` trace, and if the command has `expects == 0`, reports that the command was satisfied against expectation. Source-internal anchor: `_exec` satisfiable/unsatisfiable branches.

{8} The command-selection helper uses all commands when no `command` option is present; if the option is numeric it selects by index; otherwise it treats the value as a glob matched against command labels. Source-internal anchor: `getCommandPredicate`.

{9} The `commands` command is described as showing all commands in an Alloy program. Its implementation parses the file and prints each command with a zero-based index. Source-internal anchor: `_commands`.

{10} In table output generation, if `solution.isTemporal()` is true, the CLI prints `Trace length` and `Loop state`; for each state it prints `State index`, marks the loop state, and prints the table for that state. Source-internal anchor: `generate` table-output branch.

{11} In JSON output generation, the CLI writes the `SolutionDTO` with defaults and indentation. In XML output generation, it calls `A4SolutionWriter.writeInstance`. Source-internal anchor: `generate` JSON/XML branches.
