# Synthetic HLP fixtures

No original help-file payload is stored here. `tools/analysis/tests/test_help.py`
builds small synthetic HLP containers in memory (header, FILEHEADER, B+ index
and leaf pages, `|SYSTEM`, `|TOPIC`, optional `|Phrases`) and writes them to a
temporary directory during each test.

This directory is reserved for future committed synthetic fixtures if a static
binary fixture becomes clearer than the in-test builder.
