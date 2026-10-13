# Loaded with -r after test_prelude.rb for a run under collector stress: every allocation
# collects. The test files are loaded and run under stress; only interpreter start-up and
# the summary at exit are not.
at_exit { GC.stress = false }
GC.stress = true
