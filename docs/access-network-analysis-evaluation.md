# Access Network Analysis: YANG/NETCONF, Device Tree, Busybox

## Deep Evaluation for ROLT, vCMTS, RPD, CIN, DOCSIS 4.0, WiFi 7

**Status:** Evaluation complete
**Date:** 2026-04-02
**Context:** Charter access network platforms undergoing frequent updates with
immature codebases; DOCSIS 4.0 (FDD/TDD) and WiFi 7 (802.11be) transitions
create new analysis requirements for CPE and infrastructure code.

---

## 1. Platform Context

| Platform | Role | Key technologies | Update frequency |
|----------|------|-----------------|-----------------|
| **ROLT** (Remote OLT) | Fiber access (GPON/XGS-PON) | C/C++ data plane, YANG/NETCONF mgmt, DTS for SoC config | High — active development |
| **vCMTS** (Virtual CMTS) | DOCSIS headend (virtualized) | C++ data plane, Go/Python control, k8s orchestration, protobuf IPC | High — DOCSIS 4.0 migration |
| **RPD** (Remote PHY Device) | DOCSIS RF frontend | C firmware, DTS for FPGA/SoC, YANG/NETCONF, Busybox shell | High — DOCSIS 4.0 features |
| **CIN** (Converged Interconnect Network) | Backhaul aggregation | Go/Python SDN control, YANG/NETCONF, Terraform infra | Medium — stable but expanding |
| **CPE** (Customer Premises Equipment) | Home gateway, cable modem | C/C++ firmware, Busybox, OpenWrt, DTS, WiFi 7 drivers | Very high — WiFi 7 + DOCSIS 4.0 |

### Why these platforms need analysis now

1. **DOCSIS 4.0 transition:** FDD (Full Duplex DOCSIS) and ESD (Extended
   Spectrum DOCSIS) require fundamental changes to MAC scheduling, OFDMA
   channel management, and PHY configuration. These touch low-level C/C++
   firmware with safety-critical timing constraints.

2. **WiFi 7 (802.11be):** Multi-Link Operation (MLO), 320MHz channels, and
   4096-QAM require new driver code for Qualcomm/Broadcom/MediaTek chipsets.
   Configuration is via Device Tree overlays and nl80211 netlink interfaces.

3. **Immaturity:** These codebases are actively evolving — not stable legacy
   code. Frequent refactoring, incomplete error handling, and rapidly changing
   APIs make them high-value targets for automated review.

---

## 2. YANG / NETCONF — Deep Evaluation

### 2.1 What YANG models define

YANG (RFC 7950) defines the management data model for network devices. It
specifies:
- **Configuration data:** What can be set via NETCONF/RESTCONF
- **State data:** What can be read (operational counters, status)
- **RPCs:** Remote operations (reboot, firmware upgrade)
- **Notifications:** Async events (link down, alarm)
- **Constraints:** Must/when/leafref/unique validators

### 2.2 Where YANG appears in access network platforms

| Platform | YANG usage | Model examples |
|----------|-----------|----------------|
| ROLT | OLT management — ONU provisioning, VLAN config, traffic shaping | `bbf-xpon`, `bbf-hardware`, `ietf-interfaces` |
| vCMTS | DOCSIS service flow management, channel config | `ccap-*`, CableLabs D4.0 models |
| RPD | RF channel config, PHY parameters, DEPI tunnel management | `ccap-rpd`, `ietf-hardware` |
| CIN | Switch/router config, MPLS, QoS policies | `ietf-routing`, `openconfig-*` |
| CPE | Home gateway: WiFi, DHCP, NAT, firewall rules | `tr-385` (BBF), `ietf-system` |

### 2.3 Analysis requirements

YANG model changes in access network platforms create these review concerns:

**A. Backward compatibility:** Adding a mandatory leaf to an existing container
breaks all existing NETCONF clients. Removing or renaming a leaf breaks config
replay. Field number stability (like protobuf) but with semantic constraints.

**B. Constraint propagation:** A `when` clause on a leaf means the leaf only
exists when a condition is true. Changing the `when` clause changes which
configurations are valid — this affects deployed devices with existing config.

**C. Augmentation conflicts:** Multiple modules can `augment` the same path.
DOCSIS 4.0 models augment existing DOCSIS 3.1 models. Conflicting augmentations
cause runtime failures that aren't caught by individual model validation.

**D. Leafref integrity:** `leafref` nodes point to other leaves by XPath. If
the target path changes, all leafrefs break. This is a cross-file reference
integrity problem.

**E. Default value changes:** Changing `default` values affects devices that
rely on implicit defaults. This is a silent behavior change.

### 2.4 What analysis looks like

**Not slicing — reference graph traversal with constraint checking:**

```
Input:  YANG model diff (added/removed/modified nodes)
Step 1: Parse all .yang files, build module dependency tree (import/include)
Step 2: Resolve augmentations — which modules modify the changed path?
Step 3: Resolve leafrefs — which leafrefs point to the changed node?
Step 4: Check constraints — do when/must clauses still hold?
Step 5: Emit context: changed node + augmentations + leafrefs + constraints
```

### 2.5 Tooling assessment

| Tool | Capability | Gap |
|------|-----------|-----|
| **pyang** (Python) | Full YANG validation, tree output, UML diagram generation | Doesn't do diff-focused analysis |
| **yanglint** (C, libyang) | Runtime validation, data instance checking | Validates instances, not model diffs |
| **tree-sitter-yang** | Basic syntax tree | Immature (0.1.3), no resolution of `uses`/`grouping`/`augment` |
| **yang-rs** (Rust, 0.1.1) | Basic YANG parser | Immature, limited resolution |

**Assessment:** No mature Rust-native YANG parser exists. The best approach
would be to shell out to `pyang` for model resolution (similar to how we shell
out to `clang` for type enrichment) and parse its output. Alternatively, wrap
`libyang2` via FFI.

### 2.6 Recommendation

**Priority: Medium-High for ROLT/RPD/CIN. Build as `prism context yang`.**

Implementation approach:
1. Shell out to `pyang -f json-tree` for model resolution
2. Parse the JSON tree, build module dependency graph
3. Given a diff, identify changed nodes, trace leafrefs and augmentations
4. Emit context blocks for LLM reviewer

**Estimated effort:** 2–3 weeks (including pyang integration)
**Prerequisite:** pyang installed in CI environment

### 2.7 DOCSIS 4.0 and WiFi 7 specific YANG models

DOCSIS 4.0 introduces new YANG models for:
- **FDD channel pairs:** Full-duplex frequency allocation
- **ESD extended spectrum:** New OFDMA parameters above 1.2 GHz
- **PNM (Proactive Network Maintenance):** Spectrum analysis data model
- **Low-latency DOCSIS (LLD):** Queue management and scheduling parameters

WiFi 7 introduces YANG-modeled configuration for:
- **MLO (Multi-Link Operation):** Link aggregation, traffic steering policies
- **320MHz channels:** Channel bandwidth and guard interval configuration
- **Restricted TWT:** Target Wake Time scheduling for IoT devices

These models are evolving rapidly. Review support should focus on **backward
compatibility** (will this model change break existing deployed config?) and
**constraint consistency** (do the new when/must clauses conflict with existing
augmentations?).

---

## 3. Device Tree Source (DTS) — Deep Evaluation

### 3.1 What Device Tree defines

Device Tree (DT) describes hardware topology for Linux-based systems. It tells
the kernel what hardware exists, how it's connected, and how to configure it:

```dts
/ {
    soc {
        wifi@18000000 {
            compatible = "qcom,ipq9574-wifi";
            reg = <0x18000000 0x1000000>;
            interrupts = <GIC_SPI 20 IRQ_TYPE_LEVEL_HIGH>;
            status = "okay";
            band@0 {
                /* 6 GHz band for WiFi 7 */
                reg = <0>;
                channel-width = <320>;  /* WiFi 7: 320 MHz */
            };
        };
    };
};
```

### 3.2 Where DTS appears in access network platforms

| Platform | DTS usage | Examples |
|----------|----------|---------|
| RPD | FPGA memory maps, RF frontend SoC config, PHY register definitions | Broadcom BCM3390 SoC, FPGA overlays |
| CPE | WiFi radios, Ethernet PHYs, GPIO, I2C/SPI peripherals, flash partitions | Qualcomm IPQ9574, MediaTek MT7986 |
| ROLT | PON SoC configuration, SerDes lanes, memory regions | Broadcom BCM68xxx |

### 3.3 Analysis requirements

DTS changes in access network platforms create these review concerns:

**A. Register overlap:** Two devices with overlapping `reg` ranges cause bus
conflicts. `reg = <0x18000000 0x1000000>` and `reg = <0x18800000 0x800000>`
overlap — the second starts within the first's range.

**B. Interrupt conflicts:** Duplicate interrupt numbers cause kernel panics.
Two nodes claiming `interrupts = <GIC_SPI 20 ...>` conflict.

**C. Pin muxing conflicts:** GPIO pins configured for multiple functions.
`pinctrl` nodes define pin usage — if a WiFi 7 radio claims the same pins
as a Bluetooth controller, one fails silently.

**D. Compatible string correctness:** The `compatible` property selects the
kernel driver. A typo or wrong compatible string means no driver binds —
the device is invisible to the kernel.

**E. Status propagation:** `status = "disabled"` on a parent node disables all
children. Accidentally disabling a bus controller disables all devices on that
bus.

**F. DTS overlay conflicts:** DOCSIS 4.0 and WiFi 7 features are often added
via DTS overlays. Overlays that modify the same node can conflict if applied
in the wrong order.

### 3.4 Tooling assessment

| Tool | Capability | Gap |
|------|-----------|-----|
| **dtc** (Device Tree Compiler) | Compilation, basic warnings | No cross-reference analysis, no conflict detection |
| **dt-validate** | Schema validation against dt-bindings YAML | Validates individual nodes, not cross-node conflicts |
| **tree-sitter-devicetree** (Rust) | Basic DTS syntax tree | Exists but limited; no `#include` resolution, no overlay merging |

**Key challenge:** DTS files use C preprocessor `#include` and `/include/`
directives. A complete parse requires preprocessing first (`cpp -E` or `dtc
-I dts -O dts -P`). Tree-sitter sees the raw unexpanded source.

### 3.5 Recommendation

**Priority: Medium for RPD/CPE. Build as `prism context dts`.**

Implementation approach:
1. Preprocess DTS with `dtc -I dts -O dts -P` to resolve includes
2. Parse the flattened DTS (tree-sitter-devicetree or custom parser)
3. Build node hierarchy with `reg`, `interrupts`, `compatible`, `status` extracted
4. Given a diff, identify changed nodes, check for:
   - Register range overlaps
   - Interrupt number conflicts
   - Compatible string changes (flag for driver impact)
   - Status changes (check parent propagation)
5. Emit context blocks for reviewer

**Estimated effort:** 2–3 weeks
**Prerequisite:** `dtc` installed in CI environment

### 3.6 WiFi 7 and DOCSIS 4.0 specific DTS patterns

**WiFi 7 DTS nodes of interest:**
- `wifi` nodes with `compatible = "qcom,*-wifi"` or `mediatek,*-wifi"`
- `band@N` child nodes for multi-band/MLO configuration
- `channel-width` property (320 for WiFi 7)
- `he-*` and `eht-*` properties (HE = WiFi 6, EHT = WiFi 7 modes)
- `mlo-capable` boolean property

**DOCSIS 4.0 DTS nodes:**
- FPGA configuration for FDD echo cancellation
- RF frontend DAC/ADC configuration nodes
- Extended spectrum PHY configuration (>1.2 GHz)
- DMA channel allocation for high-throughput OFDMA

---

## 4. Busybox / Shell Scripts in Firmware — Deep Evaluation

### 4.1 Busybox context

Busybox provides a minimal Unix userspace for embedded Linux. In access
network devices, Busybox implements:
- Init system (`/sbin/init`, `/etc/init.d/`)
- Shell (`ash`, POSIX sh subset — NOT bash)
- Core utilities (`mount`, `ifconfig`, `route`, `iptables`, `udhcpc`)
- Package management hooks

### 4.2 Busybox shell vs Bash

| Feature | Bash | Busybox ash |
|---------|------|-------------|
| Arrays | Yes (`arr=(a b c)`) | No |
| `[[ ]]` test | Yes | No (use `[ ]`) |
| `local` | Yes | Yes |
| Process substitution | Yes (`<(cmd)`) | No |
| Here-strings | Yes (`<<<`) | No |
| String manipulation | `${var//pat/rep}` | Limited |
| Regex matching | `[[ $x =~ regex ]]` | No |

**Implication for Prism:** tree-sitter-bash parses both bash and POSIX sh
syntax. Busybox scripts are a subset — no additional grammar needed. However,
taint sink patterns differ slightly (Busybox `ash` has fewer builtins).

### 4.3 Firmware-specific shell script analysis

**Init scripts (`/etc/init.d/`):**
```sh
#!/bin/sh /etc/rc.common
START=50
STOP=50
USE_PROCD=1

start_service() {
    procd_open_instance
    procd_set_param command /usr/sbin/docsis_manager
    procd_set_param env CHANNEL_CONFIG="$CONFIG_FILE"  # Taint: CONFIG_FILE source?
    procd_close_instance
}
```

Analysis concerns:
- Environment variables passed to daemons — are they sanitized?
- Service ordering (START/STOP numbers) — dependencies correct?
- PID file management — race conditions?

**Firmware update scripts:**
```sh
#!/bin/sh
# sysupgrade — performs firmware update
FW_IMAGE=$1
FW_HASH=$(sha256sum "$FW_IMAGE" | cut -d' ' -f1)
EXPECTED_HASH=$(cat /etc/firmware_hash)

if [ "$FW_HASH" != "$EXPECTED_HASH" ]; then
    echo "Hash mismatch — aborting"
    exit 1
fi

mtd write "$FW_IMAGE" firmware    # Critical: wrong partition = bricked device
```

Analysis concerns:
- Hash verification before write (absence pair: hash check before mtd write)
- Correct partition name (`firmware` vs `rootfs` vs `kernel`)
- Error handling on `mtd write` failure
- Rollback mechanism presence

**Network configuration scripts:**
```sh
#!/bin/sh
# Apply VLAN configuration from TR-069 parameters
VLAN_ID=$1
INTERFACE=$2

# TAINT: $VLAN_ID and $INTERFACE from TR-069 (remote management)
vconfig add $INTERFACE $VLAN_ID      # Command injection if VLAN_ID = "; reboot"
ifconfig $INTERFACE.$VLAN_ID up      # Same vulnerability
```

### 4.4 Busybox-specific taint sinks

Beyond the general shell sinks in the Shell/Bash plan, Busybox firmware
scripts have additional high-value sinks:

| Sink | Pattern | Risk |
|------|---------|------|
| `mtd write` | Flash partition write | Device bricking |
| `fw_setenv` | U-Boot env modification | Boot-loop, persistent config corruption |
| `uci set` / `uci commit` | OpenWrt UCI config write | Persistent config injection |
| `iptables` / `ip6tables` | Firewall rule modification | Security bypass |
| `ifconfig` / `ip` | Network interface config | Network disruption |
| `brctl` / `bridge` | Bridge configuration | VLAN hopping |
| `vconfig` | VLAN configuration | Network segmentation bypass |
| `insmod` / `modprobe` | Kernel module loading | Rootkit installation |
| `mdev` triggers | Device event handlers | Privilege escalation on device plug |
| `procd_set_param env` | Service environment | Daemon config injection |
| `swconfig` | Switch chip configuration | L2 network manipulation |

### 4.5 Recommendation

**Shell/Bash support (from shell-bash-plan.md) covers Busybox scripts with
minimal additional work.** The tree-sitter-bash grammar handles POSIX sh.
The additional firmware-specific patterns (§4.4) are just more entries in
the taint sink and absence pair tables — not architectural changes.

**Additional effort beyond shell-bash-plan.md:** 1–2 days for firmware-specific
patterns and test fixtures.

---

## 5. Cross-Cutting: How These Formats Interact

Access network devices aren't isolated stacks — a single change can span
multiple formats:

```
YANG model change (new config leaf for WiFi 7 MLO)
    ↓ generates
NETCONF edit-config (applied to running config)
    ↓ triggers
Shell script (/etc/init.d/wifi restart with new params)
    ↓ reads
Device Tree overlay (MLO-specific radio configuration)
    ↓ configures
Kernel driver (ath12k/mt76 WiFi 7 driver — C code)
```

A comprehensive review of a WiFi 7 feature addition touches ALL of these.
Prism's multi-language architecture is uniquely positioned to provide context
across the full stack — YANG model context + shell script taint analysis +
DTS conflict detection + C driver code slicing.

### Cross-format reference tracing (future capability)

| Source format | Reference type | Target format |
|--------------|---------------|---------------|
| YANG model | `default` value, `when` clause | Shell script (UCI config) |
| Shell script | `uci get` / `uci set` | YANG-modeled config |
| Shell script | `mtd write`, partition name | DTS flash partition layout |
| DTS | `compatible` string | C driver `MODULE_DEVICE_TABLE` |
| DTS | `interrupts` | C driver `platform_get_irq()` |
| C driver | `of_property_read_*` | DTS property values |

This cross-format tracing is a medium-term goal (3–6 months). It requires:
1. Each format has its own analysis module (YANG, DTS, Shell, C)
2. A cross-format reference index maps identifiers across formats
3. Given a diff, trace references across format boundaries

---

## 6. Priority Matrix

| Format | Platforms | Update frequency | Analysis value | Tooling maturity | Recommended priority |
|--------|----------|-----------------|---------------|-----------------|---------------------|
| **Shell/Busybox** | All | Very high | Very high (injection) | High (tree-sitter-bash) | **P1 — implement now** |
| **YANG/NETCONF** | ROLT, RPD, CIN | High | High (compat breaks) | Medium (pyang, no Rust native) | **P2 — implement next** |
| **Device Tree** | RPD, CPE, ROLT | High | Medium-High (hw conflicts) | Low (immature grammar) | **P3 — implement after YANG** |
| **Protobuf** | vCMTS | Medium | Medium (wire compat) | High (protobuf-parse) | **P3 — implement alongside DTS** |
| Cross-format tracing | All | N/A | Very high | None (novel) | **P4 — future** |

---

## 7. Estimated Total Effort

| Item | Effort | Dependencies |
|------|--------|-------------|
| Shell/Bash (includes Busybox) | 1–2 weeks | tree-sitter-bash |
| YANG/NETCONF context extraction | 2–3 weeks | pyang in CI |
| Device Tree context extraction | 2–3 weeks | dtc in CI |
| Protobuf context extraction | 1 week | protobuf-parse crate |
| Cross-format reference index | 3–4 weeks | All above complete |
| **Total** | **~10–13 weeks** | Staggered, not sequential |

Recommended execution order: Shell → YANG → DTS → Protobuf → Cross-format.
Shell and YANG can overlap (different developers).
