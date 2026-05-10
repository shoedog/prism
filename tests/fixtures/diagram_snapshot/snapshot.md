# Prism diagram report

## Taint

### Finding 1: tainted strcpy at foo.c

```mermaid
flowchart TD
    src["foo.c:42 read"]:::source
    snk["foo.c:67 strcpy"]:::sink
    src -->|tainted| snk

```

