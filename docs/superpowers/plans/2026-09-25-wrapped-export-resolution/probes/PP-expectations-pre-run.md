# Branch-P prototype expectations (written BEFORE running; 2026-09-25)
Binary: proto-wt-p (r2 prototype minus custody K1-K8 and the mode switch; hand ESM React import parser kept), 19a1d54e.
Model: runtime mutation/acquisition of the React object is out of model (SPEC §3.2 Branch P).
C01-C27, C29, C30, C31, C32, C38, C39, C44: identical to r2(a) (P18-controls-proto-a.txt).
Custody-refused under r2 -> now Exact import_member -> lib:Island (MB class, out of model):
 C28 (=MB1), C33, C34, C35, C36, C37, C40, C41, C42, C43, C45, C46, C47, C48, C49, C50, C51, C52.
New:
 C53 (=MB2, sol r2 W1: named import + require('react').forwardRef = fake): Exact import_member -> lib.jsx:Island
 C54 (=MB3, sol r2 W3: React.__defineGetter__): Exact import_member -> lib.jsx:Island
 C55 (r2 SMELL1: value memo + `import type { T as memo }`): drop UnknownName; reason callee_provenance
 C56 (T-N18: const { forwardRef } = require('react')): drop UnknownName; reason callee_not_admitted
Stats: react_object_unaccounted never appears.
Base binary: every C53-C56 app-side site drops UnknownName.
