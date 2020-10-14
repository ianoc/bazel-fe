# Bazel runner


## Goal/motivation

Be a suite of tools to provide ultimately a different frontend around bazel. Initially this is something to be injected to wrap calls for build/test operations in bazel to repair common issues.


## Requirements
- Ability to run/provide a CI job or some means to run the index building code (Indexer itself in this repo isn't quite complete yet, eta ~10/15).
- Not be consuming the BEP from bazel directly. The tool hooks in and tells bazel to send the BEP out to it to sniff on the results.




## Using it
1) Configure a CI job to run the indexer, it should produce a binary output
2) Store the output in a location which is fetchable by your developers/users
3) From the examples you need to install:
   -> Some code/bash script (could be built into the launcher in future?) to fetch the index to provide
   -> Bash script for tools/bazel to alloow hooking into the bazel commands and delegating to the `bazel-runner` application
4) Run it

Other things:
We also include/have a small script to measure how well it can do for you/potentially handle targets with unused dependencies. the `slow_unused_deps.sh` script will remove all dependencies from a target then try build it again. If the above is all working right, hopefully like magic it should just recover + build ok.



## TODO:

[X] Bazel runner that can wrap bazel
[ ] JVM Indexer to find/index all jvm producing targets
[ ] Example project
[ ] All scripts in the right place
[ ] Integration for auto formatting handling for java/scala
[ ] Investigate persistant daemon mode:
    - [ ] When file changes rebuild the target that owns it
    - [ ] When the above is successful run tests that directly ddepend on the rebuilt target
    - [ ] Optionally run all tests that transitively depend on the target
[ ] Build UI experiments using the TUI library to show better histograms/data while building.
[ ] Web interface?
