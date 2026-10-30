// Alias-following variant matrix: enumerated fixtures with ground truth.
// Each variant is a tiny repo. The marked site (line ending in "// SITE") is inside a named
// callable so prism has a caller for it. Ground truth is what the site statically calls:
//   {t:'one', target:'file:name'}      exactly one in-repo callable
//   {t:'set', targets:[...]}           a closed set, all in-repo
//   {t:'may', targets:[...]}           possibly these, not provable from visible code
//   {t:'none'}                         no in-repo callable is the callee
//   {t:'anon', file}                   the anonymous callable on the line marked "// ANON"
// `step` says which lookup step of the owner's order would have to resolve it:
//   local | returned | param-lookup | param-args | import | import-path | same-file | global | none | anon
// Rows of step 'anon' also carry an authored `caller` truth (variants-anon.cjs).
// `contract` marks variants whose desired grade is a contract decision, not a fact.
'use strict';
const { localVariants, cjsVariants } = require('./variants-local.cjs');
const { paramVariants } = require('./variants-params.cjs');
const { importVariants } = require('./variants-imports.cjs');
const { returnVariants } = require('./variants-returns.cjs');
const { anonVariants } = require('./variants-anon.cjs');

function all() {
  return [...localVariants(), ...cjsVariants(), ...paramVariants(), ...importVariants(), ...returnVariants(), ...anonVariants()];
}
module.exports = { all };
