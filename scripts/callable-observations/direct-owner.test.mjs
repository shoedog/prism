import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync} from 'node:fs';
import path from 'node:path';
import {tmpdir} from 'node:os';
import {derive} from './detached-owner-worker.mjs';
import {produce,validate} from './index.mjs';
const compiler=process.env.PRISM_TYPESCRIPT;
assert(compiler,'explicit pinned compiler required; no silent skip');
const corpus=JSON.parse(readFileSync(new URL('../../docs/eval/receiver-closure/executable-owner-fixtures.json',import.meta.url)));

function fixture(change,run,extension='ts') {
  const root=mkdtempSync(path.join(tmpdir(),'prism-direct-owner-'));
  const files={...corpus.files,...change.files};
  for(const file of change.remove??[])delete files[file];
  if(change.replace_app){const [before,after]=change.replace_app;assert.equal(files['src/app.ts'].split(before).length,2);files['src/app.ts']=files['src/app.ts'].replace(before,after);}
  const app=`src/app.${extension}`;
  if(extension==='tsx'){files[app]=files['src/app.ts'];delete files['src/app.ts'];}
  const options={...corpus.compiler_options,...change.options};
  for(const name of change.delete_options??[])delete options[name];
  const put=(file,source)=>{mkdirSync(path.dirname(path.join(root,file)),{recursive:true});writeFileSync(path.join(root,file),source);};
  for(const [file,source] of Object.entries(files))put(file,source);
  put('package.json','{"type":"module"}');put('tsconfig.json',JSON.stringify({compilerOptions:options,include:['src']}));
  const input={root,compiler,config:'tsconfig.json'};
  try {return run(derive(input),{input,files,app,put});} finally {rmSync(root,{recursive:true,force:true});}
}
for(const extension of ['ts','tsx'])for(const c of corpus.cases) {
  test(`detached design corpus ${c.id}/${extension}`,()=>fixture(c,(r,{files})=>{
    assert.equal(r.packet.semantic_closure.complete,c.semantic_complete);
    assert.equal(r.packet.authorizes_runtime_edge,false);
    const eligible=r.packet.semantic_closure.complete && r.candidates.length>0;
    // The compiler pass cannot know the separate Prism index. Rust tests refuse
    // this extra input; retain its complete compiler Program as the control.
    assert.equal(eligible,['candidate','different_genuine_input','out_of_program_effect'].includes(c.id),JSON.stringify(r.refusals));
    if(c.id==='out_of_program_effect') {
      assert(Object.hasOwn(files,'outside/effect.ts'));
      assert(!r.sources.some(s=>s.file==='outside/effect.ts'));
    }
  },extension));
}

const app=corpus.files['src/app.ts'],contract=corpus.files['src/contract.ts'],client=corpus.files['src/client.ts'];
const negative=[
  ['let',{replace_app:['export const run','export let run']},'implementation'],
  ['multiple_declarators',{replace_app:['export const run','export const marker = 1, run']},'implementation'],
  ['async',{replace_app:['= ({ client })','= async ({ client })']},'implementation'],
  ['function_expression',{replace_app:['({ client }) =>','function ({ client })']},'implementation'],
  ['return_annotation',{replace_app:['({ client }) =>','({ client }): void =>']},'implementation'],
  ['parameter_default',{replace_app:['({ client })','({ client } = {client: new Client()})']},'implementation'],
  ['binding_default',{replace_app:['({ client })','({ client = new Client() })']},'binding'],
  ['binding_rename',{files:{'src/app.ts':app.replace('({ client })','({ client: other })').replace('client.m()','other.m()')}},'binding'],
  ['binding_rest',{files:{'src/app.ts':app.replace('({ client })','({ ...rest })').replace('client.m()','rest.client.m()')}},'binding'],
  ['optional_call',{replace_app:['client.m()','client.m?.()']},'direct_call'],
  ['computed_call',{replace_app:['client.m()',"client['m']()"]},'direct_call'],
  ['call_arguments',{replace_app:['client.m()','client.m(1)'],files:{'src/client.ts':client.replace('m()','m(x?: number)')}},'direct_call'],
  ['callable_value_import',{replace_app:['import type { Handler }','import { Handler }']},'callable'],
  ['callable_renamed',{files:{'src/app.ts':app.replace('{ Handler }','{ Handler as Other }').replace('Handler<Props>','Other<Props>')}},'callable'],
  ['callable_namespace',{files:{'src/app.ts':app.replace('{ Handler }','* as C').replace('Handler<Props>','C.Handler<Props>')}},'callable'],
  ['callable_barrel',{replace_app:["from './contract'","from './barrel'"],files:{'src/barrel.ts':"export type {Handler} from './contract';\n"}},'callable'],
  ['binder_constraint',{files:{'src/contract.ts':contract.replace('<P>','<P extends object>')}},'callable'],
  ['binder_default',{files:{'src/contract.ts':contract.replace('<P>','<P = unknown>')}},'callable'],
  ['two_binders',{files:{'src/contract.ts':contract.replace('<P>','<P, Q = unknown>')}},'callable'],
  ['callable_nonvoid',{files:{'src/contract.ts':contract.replace('=> void','=> unknown')}},'callable'],
  ['partial_binder',{files:{'src/contract.ts':"import {Client} from './client'; export type Handler<P> = (props: {client: Client}) => void;\n"}},'substitution'],
  ['optional_signature',{files:{'src/contract.ts':contract.replace('props: P','props?: P')}},'substitution'],
  ['rest_signature',{files:{'src/contract.ts':contract.replace('props: P','...props: P[]')}},'substitution'],
  ['props_interface',{replace_app:['type Props = { client: Client };','interface Props { client: Client }']},'props'],
  ['props_exported',{replace_app:['type Props =','export type Props =']},'props'],
  ['props_generic',{replace_app:['type Props =','type Props<T = string> =']},'props'],
  ['props_readonly',{replace_app:['{ client: Client }','{ readonly client: Client }']},'property'],
  ['props_extra',{replace_app:['{ client: Client }','{ client: Client; other?: number }']},'props'],
  ['props_intersection',{replace_app:['{ client: Client };','{ client: Client } & {};']},'props'],
  ['props_computed',{replace_app:['{ client: Client }',"{ ['client']: Client }"]},'property'],
  ['class_type_import',{replace_app:['import { Client }','import type { Client }']},'class'],
  ['class_abstract',{files:{'src/client.ts':client.replace('export class','export abstract class')}},'class'],
  ['class_default',{files:{'src/client.ts':client.replace('export class','export default class'),'src/app.ts':app.replace('{ Client }','Client')}},'class'],
  ['member_generic',{files:{'src/client.ts':client.replace('m()','m<T>()')}},'member'],
  ['member_async',{files:{'src/client.ts':client.replace('m(): number','async m(): Promise<number>')}},'member'],
  ['member_accessor',{files:{'src/client.ts':"export class Client { get m() {return () => 1;} }\n"}},'member'],
  ['computed_slot',{files:{'src/client.ts':client.replace('m():','[Symbol.toStringTag] = "Client"; m():')}},'member'],
  ['this_write',{files:{'src/client.ts':client.replace('  m()', '  constructor() { this.m = () => 2; }\n  m()')}},'member_write'],
  ['reflective_effect',{files:{'src/effect.ts':"import {Client} from './client'; Object.defineProperty(Client, 'm', {value:()=>2});\n"}},'indexed_effect'],
  ['prototype_read',{files:{'src/effect.ts':"import {Client} from './client'; const x = Client.prototype;\n"}},'indexed_effect'],
  ['class_binding_write',{files:{'src/client.ts':client+"Client = class { m(): number {return 2;} };\n"}},'binding_write'],
  ['run_binding_write',{files:{'src/app.ts':app+"run = ({client}) => { client.m(); };\n"}},'binding_write'],
];
for(const [id,change,reason] of negative)test(`direct predicate refuses ${id}`,()=>fixture(change,r=>{
  assert.equal(r.candidates.length,0,`${id}: candidate must be refused independent of closure`);
  assert(r.refusals.some(row=>row.reason===reason),`${id}: ${JSON.stringify(r.refusals)}`);
}));

for(const [id,source] of [
  ['unicode','// 🦊\r\n'+app],['bom','\ufeff'+app],['crlf',app.replaceAll('\n','\r\n')],
  ['non_magic_names',app.replaceAll('Handler','Callable').replaceAll('Props','Inputs').replaceAll('Client','Owner').replaceAll('client','receiver').replaceAll('.m()', '.work()')],
])test(`direct positive ${id} binds original source`,()=>fixture({files:{'src/app.ts':source,
  ...(id==='non_magic_names'?{'src/contract.ts':contract.replaceAll('Handler','Callable'),'src/receiver.ts':client.replaceAll('Client','Owner').replace('m()', 'work()')}:{})},
  ...(id==='non_magic_names'?{remove:['src/client.ts']}:{})},r=>{
  assert.equal(r.packet.semantic_closure.complete,true,JSON.stringify(r.packet.diagnostics));
  assert.equal(r.candidates.length,1,JSON.stringify(r.refusals));
  const c=r.candidates[0];assert.equal(Object.keys(c.anchors).length,21);
  if(id==='unicode')assert(c.anchors.call.start_byte>c.anchors.call.start_utf16);
}));

test('ordinary constructor remains supported',()=>fixture({files:{'src/client.ts':client.replace('  m()', '  constructor() {}\n  m()')}},r=>assert.equal(r.candidates.length,1)));
test('factory hook preserves ordinary observer fields and reproduction',()=>fixture({},(r,{input})=>{
  assert.deepEqual(r.packet,produce(input));
  assert.equal(validate(JSON.stringify(r.packet),input).valid,true);
  assert.equal(Object.hasOwn(r.packet.observations[0].calls[0],'props_class'),false);
}));
test('two separately derived genuine source inputs retain distinct class identity',()=>{
  const a=fixture({},r=>r),b=fixture({files:{'src/client.ts':client.replace('return 1','return 2')}},r=>r);
  assert.deepEqual(a.candidates[0].anchors.call,b.candidates[0].anchors.call);
  assert.notEqual(a.candidates[0].anchors.class_declaration.sha256,b.candidates[0].anchors.class_declaration.sha256);
  assert.notDeepEqual(a,b);
});
