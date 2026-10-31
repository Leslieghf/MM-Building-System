# Xref probe

Windows-only, read-only scanner used by the world-query research notes. It
searches the main executable image for relative calls and absolute pointers to
the catalogued target RVAs.

```text
trace_xref_probe <PID>
trace_xref_probe <PID> --image-range <base-hex> <size-hex>
trace_xref_probe <PID> --peek <image-address-hex> [length]
trace_xref_probe <PID> --peek-any <process-address-hex> [length]
```

The process handle requests query and memory-read access only. Peek lengths are
limited to 256 bytes. `--peek` is restricted to the `MadMax.exe` image;
`--peek-any` is for explicitly selected process addresses. The tool does not
write process memory or launch the game.
