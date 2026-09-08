import path from 'node:path';
import {canonical,hash,relative} from './schema.mjs';

export const CONFIG_OPTION_NAMES=['types','lib','typeRoots','noLib','libReplacement','noResolve','target'];
const REASONS=['chain_limit','unavailable','invalid_config','duplicate_property','unsupported_extends','cycle','missing_target','selection_mismatch','unsupported_option','value_mismatch'];
const empty=reason=>({status:'unproven',reason,files:[],extends:[],options:[]});

// Drop-in extendedConfigCache: reads always miss and writes can never change parsing.
export function createConfigCapture({cap=33}={}) {
  if(!Number.isSafeInteger(cap)||cap<1)throw Error('invalid_capture_cap');
  const capturedRecords={records:[],writes:0,overflow:false,cap};
  const cache={capturedRecords,get(){return undefined;},set(key,value){
    capturedRecords.writes++;
    if(capturedRecords.records.length<cap)capturedRecords.records.push({key,value});
    else capturedRecords.overflow=true;
    return cache;
  }};
  return cache;
}

const decodedName=(ts,node)=>ts.isStringLiteral(node)||ts.isNumericLiteral(node)?node.text:null;
function literalValue(ts,node) {
  if(ts.isStringLiteral(node))return node.text;
  if(ts.isNumericLiteral(node))return Number(node.text);
  if(node.kind===ts.SyntaxKind.TrueKeyword)return true;
  if(node.kind===ts.SyntaxKind.FalseKeyword)return false;
  if(node.kind===ts.SyntaxKind.NullKeyword)return null;
  if(ts.isPrefixUnaryExpression(node)&&node.operator===ts.SyntaxKind.MinusToken&&ts.isNumericLiteral(node.operand))return -Number(node.operand.text);
  if(ts.isArrayLiteralExpression(node))return node.elements.map(element=>literalValue(ts,element));
  if(ts.isObjectLiteralExpression(node)) {
    const value={};
    for(const property of node.properties) {
      if(!ts.isPropertyAssignment(property))throw Error('unsupported_json');
      const name=decodedName(ts,property.name);if(name===null)throw Error('unsupported_json');
      value[name]=literalValue(ts,property.initializer);
    }
    return value;
  }
  throw Error('unsupported_json');
}

function inspectSource(ts,source) {
  if(!source||typeof source.text!=='string'||typeof source.fileName!=='string')return {problem:'unavailable'};
  if(source.parseDiagnostics?.length)return {problem:'invalid_config'};
  const statement=source.statements?.length===1&&source.statements[0];
  const object=statement&&ts.isExpressionStatement(statement)&&ts.isObjectLiteralExpression(statement.expression)?statement.expression:null;
  if(!object)return {problem:'invalid_config'};
  const roots=new Map(),duplicates=[];
  for(const property of object.properties) {
    if(!ts.isPropertyAssignment(property))return {problem:'invalid_config'};
    const name=decodedName(ts,property.name);if(name===null)return {problem:'invalid_config'};
    if(roots.has(name))duplicates.push(name);roots.set(name,property);
  }
  let compilerObject=null,compilerRaw={},optionProperties=new Map();
  const compiler=roots.get('compilerOptions');
  if(compiler) {
    if(!ts.isObjectLiteralExpression(compiler.initializer))return {problem:'unsupported_option'};
    compilerObject=compiler.initializer;
    for(const property of compilerObject.properties) {
      if(!ts.isPropertyAssignment(property))return {problem:'unsupported_option'};
      const name=decodedName(ts,property.name);if(name===null)return {problem:'unsupported_option'};
      if(optionProperties.has(name))duplicates.push(`compilerOptions.${name}`);
      optionProperties.set(name,property);
      try{compilerRaw[name]=literalValue(ts,property.initializer);}catch{return {problem:'unsupported_option'};}
    }
  }
  const extension=roots.get('extends');
  return {source,roots,duplicates,compilerRaw,optionProperties,extension};
}

const utf8Offset=(text,position)=>Buffer.byteLength(text.slice(0,position));
function nodeAnchor(source,node,file,record,kind) {
  const start=node.getStart(source),end=node.end;
  return {file,sha256:record.sha256,kind,start_utf16:start,end_utf16:end,start_byte:utf8Offset(source.text,start),end_byte:utf8Offset(source.text,end)};
}
function fullAnchor(source,file,record) {
  return {file,sha256:record.sha256,kind:'SourceFile',start_utf16:0,end_utf16:source.text.length,start_byte:0,end_byte:Buffer.byteLength(source.text)};
}
const hasConfigDir=value=>typeof value==='string'?/^\$\{configDir\}/i.test(value):Array.isArray(value)?value.some(hasConfigDir):value&&typeof value==='object'?Object.values(value).some(hasConfigDir):false;
const validOptionValue=(ts,name,value)=>name==='types'||name==='lib'||name==='typeRoots'
  ?Array.isArray(value)&&value.every(item=>typeof item==='string')
  :name==='target'?Number.isSafeInteger(value)&&Object.values(ts.ScriptTarget).includes(value)&&value!==ts.ScriptTarget.JSON
  :typeof value==='boolean';

// Pure consumer of already-captured compiler data. configDiagnostics is required
// because semantic config errors are not all present on the JSON SourceFiles;
// caseSensitive is the worker's measured host policy for the lexical cache key.
export function observeConfigProvenance({ts,source,effectiveOptions,capturedRecords,canonicalId,configReadMembership,inventory,configDiagnostics,caseSensitive}) {
  if(!ts||!source||!effectiveOptions||!capturedRecords||typeof canonicalId!=='function'
      ||!(configReadMembership instanceof Set)||!(inventory instanceof Map)||!Array.isArray(configDiagnostics)
      ||typeof caseSensitive!=='boolean')return empty('unavailable');
  const problems=new Set();
  if(capturedRecords.overflow)problems.add('chain_limit');
  if(configDiagnostics.length)problems.add('invalid_config');
  const records=Array.isArray(capturedRecords.records)?capturedRecords.records:[];
  if(!Number.isSafeInteger(capturedRecords.writes)||capturedRecords.writes<records.length)problems.add('unavailable');
  const used=new Set(),chain=[],edges=[],seen=new Set();
  let current=source;
  const idOf=file=>{try{const id=canonicalId(file);return typeof id==='string'&&relative(id)&&id.startsWith('project/')?id:null;}catch{return null;}};
  const exists=file=>{const id=idOf(file);return id!==null&&inventory.has(id);};
  while(current) {
    const inspected=inspectSource(ts,current);if(inspected.problem){problems.add(inspected.problem);break;}
    const id=idOf(current.fileName),record=id&&inventory.get(id);
    if(!id||!record||!configReadMembership.has(id)||record.size!==Buffer.byteLength(current.text)||record.sha256!==hash(current.text)){
      problems.add('selection_mismatch');break;
    }
    if(seen.has(id)){problems.add('cycle');break;}seen.add(id);
    chain.push({...inspected,id,record,full:fullAnchor(current,id,record)});
    if(chain.length>32){problems.add('chain_limit');break;}
    if(inspected.duplicates.length)problems.add('duplicate_property');
    if(!inspected.extension)break;
    const initializer=inspected.extension.initializer;
    if(!ts.isStringLiteral(initializer)){problems.add('unsupported_extends');break;}
    const spelling=initializer.text;
    if(!/^\.\.?(?:\/|$)/.test(spelling)||spelling.includes('\\')||hasConfigDir(spelling)){
      problems.add('unsupported_extends');break;
    }
    const candidate=path.posix.normalize(path.posix.resolve(path.posix.dirname(current.fileName),spelling));
    if(!(candidate==='/__prism__/project'||candidate.startsWith('/__prism__/project/'))){problems.add('unsupported_extends');break;}
    const selected=exists(candidate)?candidate:!candidate.endsWith('.json')&&exists(candidate+'.json')?candidate+'.json':null;
    if(!selected){problems.add('missing_target');break;}
    const selectedId=idOf(selected);
    if(selectedId&&seen.has(selectedId)){problems.add('cycle');break;}
    const cacheKey=ts.createGetCanonicalFileName(caseSensitive)(selected);
    const matches=records.map((record,index)=>({record,index,key:record?.key,file:record?.value?.extendedResult?.fileName}))
      .filter(item=>!used.has(item.index)&&typeof item.key==='string'&&typeof item.file==='string'
        &&item.key===cacheKey&&item.file===selected);
    if(matches.length!==1){problems.add('selection_mismatch');break;}
    const match=matches[0],extended=match.record.value?.extendedResult;
    used.add(match.index);
    if(typeof extended?.text!=='string'){problems.add('missing_target');break;}
    if(extended.parseDiagnostics?.length){problems.add('invalid_config');break;}
    if(!match.record.value?.extendedConfig?.options){problems.add('selection_mismatch');break;}
    edges.push({source:nodeAnchor(current,initializer,id,record,'StringLiteral'),selected,selectedId});
    current=extended;
  }
  if(capturedRecords.writes!==records.length||used.size!==records.length
      ||configReadMembership.size!==chain.length||chain.some(entry=>!configReadMembership.has(entry.id)))problems.add('selection_mismatch');
  const options=[];
  if(chain.length&&![...problems].some(reason=>REASONS.indexOf(reason)<=REASONS.indexOf('selection_mismatch'))) {
    for(const entry of chain) {
      if(hasConfigDir(entry.compilerRaw))problems.add('unsupported_option');
      const converted=ts.convertCompilerOptionsFromJson(entry.compilerRaw,path.posix.dirname(entry.source.fileName),entry.source.fileName);
      entry.converted=converted.options;if(converted.errors.length)problems.add('unsupported_option');
    }
    for(const name of CONFIG_OPTION_NAMES) {
      const entry=chain.find(item=>item.optionProperties.has(name)),present=!!entry;
      const value=present?entry.converted?.[name]:null,actual=effectiveOptions[name];
      if(present&&!validOptionValue(ts,name,value))problems.add('unsupported_option');
      if(present&&canonical(value)!==canonical(actual)||!present&&actual!==undefined)problems.add('value_mismatch');
      options.push({name,present,value_sha256:hash(canonical({present,value:present?actual:null})),
        origin:present?nodeAnchor(entry.source,entry.optionProperties.get(name),entry.id,entry.record,'PropertyAssignment'):null});
    }
  }
  const reason=REASONS.find(candidate=>problems.has(candidate));
  if(reason)return empty(reason);
  const byId=new Map(chain.map(entry=>[entry.id,entry.full]));
  return {status:'observed',reason:null,files:chain.map(entry=>entry.full),
    extends:edges.map(edge=>({source:edge.source,target:byId.get(edge.selectedId)})),options};
}
