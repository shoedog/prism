"""R1 closed findings: negative and edge controls, no network acquisition."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time
from types import SimpleNamespace as NS

import pytest

from tier_a.model import FunctionDef, Location, CallEdge, DefTarget
from tier_a.oracles import OracleError, OracleTimeout
from tier_a.sut import SutAmbiguous, SutTimeout, SutError


def test_configuration_array_preserves_features_and_sections():
    from tier_a.lsp_client import LspClient
    c = LspClient(['fake'], '.', initialization_options={'cargo': {'features': ['mcp']}})
    sent=[]; c._write=sent.append
    c._dispatch({'id': 3, 'method': 'workspace/configuration', 'params': {'items': [
        {'section':'rust-analyzer'}, {'section':'rust-analyzer.cargo.features'}, {'section':'unknown'}]}})
    assert sent == [{'jsonrpc':'2.0','id':3,'result':[{'cargo':{'features':['mcp']}},['mcp'],{}]}]


def test_readiness_requires_status_and_rechecks_after_inventory():
    from tier_a.oracles import LspOracle
    o=LspOracle(['fake'], '.', 'rust', settle_s=0, startup_timeout_s=.02)
    o.client.deadline=time.monotonic()+1
    messages=[{'method':'experimental/serverStatus','params':{'quiescent':False}},
              {'method':'experimental/serverStatus','params':{'quiescent':True}}]
    o.client.drain_notifications=lambda: [messages.pop(0)] if messages else []
    o.wait_ready(time.monotonic()+.3)
    assert o._quiescent
    o.client.drain_notifications=lambda: [{'method':'experimental/serverStatus','params':{'quiescent':False}}]
    with pytest.raises(OracleTimeout): o.wait_ready(time.monotonic()+.02)


@pytest.mark.parametrize('failure,expected',[(SutTimeout('hung'),'sut_timeout'),(SutError('broken'),'sut_error')])
def test_ambiguous_pin_failure_is_unavailable(failure,expected):
    from tier_a import pinned
    def fail(*args): raise failure
    rows=pinned.run_pinned(NS(),NS(callers_by_symbol=fail),[],'.')
    assert rows[-1]['outcome']==expected and rows[-1]['error']==str(failure)
    def ambiguity(*args): raise SutAmbiguous('two candidates')
    assert pinned.run_pinned(NS(),NS(callers_by_symbol=ambiguity),[],'.')[-1]['outcome']=='ok'


@pytest.mark.parametrize('stage',['edges','stats'])
@pytest.mark.parametrize('mode',['expired','hung'])
def test_dfg_both_commands_share_deadline(monkeypatch,stage,mode):
    from tier_a.matrix import load_case,_run_dfg_case
    case=load_case(Path(__file__).parents[1]/'fixtures/python/dfg_reaching_loop_carried/expected.toml')
    sut=NS(bin='fake',query_timeout_s=.02,deadline=time.monotonic()+1)
    launched=[]
    if stage=='edges' and mode=='expired': sut.deadline=time.monotonic()-1
    def run(cmd,**kw):
        launched.append(cmd)
        assert 0 < kw['timeout'] <= .02
        if stage=='edges' or '--edges' not in cmd: raise subprocess.TimeoutExpired(cmd,kw['timeout'])
        if mode=='expired': sut.deadline=time.monotonic()-1
        # Valid JSONL; reach stats seam regardless of edge-match result.
        return NS(returncode=0,stdout='{}\n')
    monkeypatch.setattr(subprocess,'run',run)
    with pytest.raises(SutTimeout): _run_dfg_case(case,'python',sut)
    assert len(launched)==(0 if stage=='edges' and mode=='expired' else 1 if stage=='edges' or mode=='expired' else 2)


def test_position_preserves_literals_comments_and_utf16():
    from tier_a.spotcheck import find_call_position
    text='const q="😀 target()"; /* target() */ target();'
    pos=find_call_position(text,'target')
    assert pos==text.rindex('target')
    from tier_a.spotcheck import utf16_column
    assert utf16_column(text,pos)==pos+1
    assert find_call_position('// target()', 'target') is None


def test_manifest_allows_ancestor_symlink_but_rejects_inner_and_dependencies(tmp_path):
    from tier_a.corpus import verify_source_manifest
    real=tmp_path/'real';real.mkdir();(real/'a.ts').write_text('function a() {}')
    ancestor=tmp_path/'link';ancestor.symlink_to(real,target_is_directory=True)
    manifest=tmp_path/'manifest';manifest.write_text(hashlib.sha256((real/'a.ts').read_bytes()).hexdigest()+'  a.ts\n')
    cfg={'path':str(ancestor),'source_manifest':str(manifest),'source_manifest_sha256':hashlib.sha256(manifest.read_bytes()).hexdigest(),'reject_node_modules':True}
    # Corpus root itself must be real; a symlink ABOVE it is allowed.
    root=real/'nested'/'source';root.mkdir(parents=True);shutil.copy(real/'a.ts',root/'a.ts');cfg['path']=str(ancestor/'nested'/'source')
    assert verify_source_manifest(cfg)[1]==[]
    (root/'node_modules').mkdir()
    assert 'oracle_node_modules_present:node_modules' in verify_source_manifest(cfg)[1]
    (root/'a.ts').unlink();(root/'a.ts').symlink_to(real/'a.ts')
    assert 'source_symlink:a.ts' in verify_source_manifest(cfg)[1]


def test_tsserver_startup_cap_is_applied():
    from tier_a.tsserver import TsserverOracle
    o=TsserverOracle([sys.executable,'-c','import time;time.sleep(30)'],'.','ts',startup_timeout_s=.025,query_timeout_s=.4)
    start=time.monotonic()
    try:
        with pytest.raises(OracleTimeout): o.start()
        assert time.monotonic()-start < .2
    finally: o.stop()


def test_unpreparable_is_error_not_empty():
    from tier_a.tsserver import TsserverOracle
    o=TsserverOracle(['fake'],'.','ts');o._open=lambda *a:None
    o._req=lambda *a,**k:[]
    fd=FunctionDef('f','function',None,Location('a.ts',1,1),1)
    with pytest.raises(OracleError,match='no item'):o.callers(fd)


@pytest.mark.parametrize('exc,label',[(FileNotFoundError('manifest'),'input_unavailable'),(ValueError('malformed'),'input_unavailable'),(SutError('inventory'),'sut_error')])
def test_lifecycle_failure_label(exc,label):
    from tier_a.cli import invalid_corpus_run
    result=invalid_corpus_run('x',{'path':'/nonexistent','oracle':'fake'},{},NS(date='test'),exc)
    assert result['meta']['invalid_reasons']==[label]


def test_ts_js_lexical_stratum_and_sample_selection():
    from tier_a.strata import is_nested
    from tier_a.cli import select_corpora
    fd=FunctionDef('f','function',None,Location('deep/a.ts',1,1),1)
    assert not is_nested(fd,'ts')
    assert is_nested(FunctionDef('f','function','outer',fd.location,1),'js')
    cfg={'quick':{'corpora':['r','t','j']},'corpus':{'r':{'lang':'rust'},'t':{'lang':'ts'},'j':{'lang':'js'}}}
    assert select_corpora(cfg,NS(corpus=None,lang='ts,js',quick=False))==['t','j']


def test_native_static_binding_quoted_offsets_and_calibration(tmp_path):
    from tier_a.tsserver import TsserverOracle
    source='''export function target() {}\nexport class Base { m() {} }\nexport class Sub extends Base { m() { super.m(); } "quoted"() { target(); } get value() { return 1; } }\nexport const obj={ perform: () => target() };\nexport function use(){ const q="😀abcdefgh"; target(); new Base().m(); new Sub().m(); obj.perform(); return new Sub().value; }\nconst alias=target; export function viaAlias(){ alias(); }\nconst {perform}=obj; export function destructured(){ perform(); }\nexport const named=function inner(){target();};\n'''
    (tmp_path/'a.ts').write_text(source)
    (tmp_path/'b.ts').write_text('import {target} from "./a"; export function across(){target();}')
    o=TsserverOracle(['tsserver','--disableAutomaticTypingAcquisition'],str(tmp_path),'ts')
    try:
        o.start(); inv=o.document_symbols('a.ts')+o.document_symbols('b.ts')
        sub=next(fd for fd in inv if fd.name=='m' and fd.container=='Sub')
        edges=o.callers(sub)
        assert len(edges)==1 and edges[0].call_site.start_line==5
        assert o.oracle_filtered  # base and super calls do not bind to Sub.m
        assert 'quoted' in {fd.name for fd in inv}
        from tier_a.spotcheck import find_call_position,utf16_column
        line=source.splitlines()[4]; col=find_call_position(line,'target')
        defs=o.definitions_at(inv,'a.ts',5,utf16_column(line,col))
        assert any(d.name=='target' for d in defs)
        from tier_a.member_sample import syntax_census,sample_members
        census=syntax_census(o,['a.ts','b.ts'])
        assert {'property_callable','getter'} <= {d['shape'] for d in census['declarations']}
        fd=next(fd for fd in inv if fd.name=='target')
        direct=o.callers(fd)
        # Keep native omissions visible as calibration data, not blessed edges.
        assert any(e.call_site.file=='b.ts' for e in direct)
        assert 'perform' not in {fd.name for fd in inv}
        pfd=FunctionDef('perform','arrow_function',None,Location('a.ts',4,4),4)
        getter=FunctionDef('value','method_definition','Sub',Location('a.ts',3,3),3)
        member=sample_members(o,NS(callers=lambda *a:[]),[pfd,getter],census,20,42)
        assert member['shapes']['property_callable']['missing']>=1
        assert member['shapes']['getter']['missing']==1
    finally:o.stop()


def _runner(monkeypatch,tmp_path,*,pin_failure=None,sut_failure=False,lang='rust',version='fake',expected_version='fake',seeds_override=None,default_per=16,prime_errors=None):
    from tier_a import cli
    seeds=[FunctionDef('f'+str(i),'function',None,Location('src/lib.rs' if lang=='rust' else 'a.ts',i+1,i+1),i+1) for i in range(20)]
    if seeds_override is not None:
        seeds=seeds_override
    class Oracle:
        not_quiescent=False
        retries=[]
        queries=0
        def start(self):pass
        def stop(self):pass
        def version(self):return version
        def capability_probe(self):return True
        def document_symbols(self,*a):return seeds
        def callers(self,*a):
            self.queries+=1
            if prime_errors is not None and self.queries<=prime_errors:
                raise OracleError('no item')
            return []
        def callees(self,*a):return []
    if prime_errors is not None:
        from tier_a.oracles import LspOracle
        Oracle.prime_hierarchy=LspOracle.prime_hierarchy
    calls=[]
    def sut_query(*a):
        calls.append(a)
        if sut_failure and len(calls)==1:raise SutTimeout('one of 32 timed out')
        return []
    def pin(*a):
        if pin_failure:raise pin_failure
        raise SutAmbiguous('correct safe failure')
    sut=NS(sha='abc',dirty=False,inventory=lambda *a:seeds,callers=sut_query,callees=sut_query,callers_by_symbol=pin)
    monkeypatch.setattr(cli,'PrismCli',lambda *a,**k:sut)
    monkeypatch.setattr(cli,'make_oracle',lambda c:Oracle())
    monkeypatch.setattr(cli,'corpus_sha',lambda p:'abc')
    monkeypatch.setattr(cli,'corpus_dirty',lambda p:False)
    monkeypatch.setattr(cli,'untracked_sources',lambda *a:[])
    monkeypatch.setattr(cli,'universe',lambda *a,**k:[seeds[0].location.file])
    monkeypatch.setattr(cli,'snapshot_path',lambda *a:tmp_path/'snap')
    monkeypatch.setattr(cli,'run_matrix',lambda *a:[])
    monkeypatch.setattr(cli.pinned_mod,'PINNED',[cli.pinned_mod.PINNED[-1]])
    if lang=='ts':
        try:
            from tier_a import member_sample
        except ImportError:
            pass  # base has no supplemental sample; behavioral checks still run
        else:
            monkeypatch.setattr(member_sample,'syntax_census',lambda *a:{'declarations':[],'sites':[]})
    return cli.run_corpus('tiny',{'path':str(tmp_path),'lang':lang,'oracle':'tsserver' if lang=='ts' else 'rust-analyzer',
                         'pinned_probes':True,'oracle_version':expected_version},
          {'seed':42,'per_stratum':default_per,'oracle_error_floor':{lang:.1},'sut_error_floor':.05},
          NS(sut_bin=None,allow_stale_sut=True,quick=False,sample=16,date='test'))


@pytest.mark.parametrize('failure',[SutTimeout('only pin timed out'),SutError('only pin failed')])
def test_runner_pinned_failure_invalidates_and_counts(monkeypatch,tmp_path,failure):
    result=_runner(monkeypatch,tmp_path,pin_failure=failure)
    assert result['meta']['baseline_invalid'] and result['meta']['sut_error_rate']>0
    assert result['failures']['pinned:ambiguous-symbol-contract']['outcome'].startswith('sut_')
    assert not any(p['outcome']=='regression' for p in result['pinned'])


def test_nonquick_timeout_below_error_floor_still_invalid(monkeypatch,tmp_path):
    result=_runner(monkeypatch,tmp_path,sut_failure=True)
    assert result['meta']['sut_error_rate']<.05
    assert result['meta']['baseline_invalid'] and 'sut_timeout' in result['meta']['invalid_reasons']
    assert result['meta']['sample_per_stratum']==16
    assert len([p for p in result['probes'] if not p.startswith('_')])==32
    assert result['probes']['_strata']['U-free']['population_symbols']==20


def test_oracle_version_drift_invalidates(monkeypatch,tmp_path):
    result=_runner(monkeypatch,tmp_path,lang='ts',expected_version='old')
    assert result['meta']['baseline_invalid'] and 'oracle_version_drift' in result['meta']['invalid_reasons']
    assert result['meta']['oracle_environment']['oracle']=='fake'
    assert result['meta']['corpus_identity']['oracle_environment_sha256']


def test_positive_full_run_and_member_shape_empty_or_error(monkeypatch,tmp_path):
    assert not _runner(monkeypatch,tmp_path)['meta']['baseline_invalid']
    from tier_a.member_sample import sample_members
    def fail(*a):raise OracleTimeout('late definition')
    census={'declarations':[{'file':'a.ts','name':'perform','shape':'property_callable','line':1}],
            'sites':[{'file':'a.ts','name':'perform','kind':'call','line':2,'character':0}]}
    result=sample_members(NS(root='.',raw_definitions_at=fail),NS(),[],census,10,42)
    assert result['shapes']['property_callable']['exclusions']=={'oracle_timeout':1}
    assert result['shapes']['getter']['sampled_sites']==0
    assert result['shapes']['getter']['detection_recall'] is None


@pytest.mark.parametrize('offset,expected',[(21,'beta'),(99,None)])
def test_member_binding_preserves_same_line_native_column(tmp_path,offset,expected):
    from tier_a.member_sample import sample_members
    declarations=[{'file':'a.ts','line':1,'character':col,'name':name,'shape':'property_callable'}
                  for name,col in [('alpha',5),('beta',20)]]
    census={'declarations':declarations,'sites':[{'file':'a.ts','line':2,'character':0,'name':'beta','kind':'call'}]}
    oracle=NS(root=str(tmp_path),raw_definitions_at=lambda *a:[{'file':str(tmp_path/'a.ts'),
                'start':{'line':1,'offset':offset},'end':{'line':1,'offset':offset+4}}])
    queried=[]
    sut=NS(callers=lambda root,fd:queried.append(fd.name) or [])
    inventory=[FunctionDef(d['name'],'arrow_function',None,Location('a.ts',1,1),1) for d in declarations]
    result=sample_members(oracle,sut,inventory,census,10,42)['shapes']['property_callable']
    assert queried==([] if expected is None else [expected])
    assert result['missing']==int(expected is not None)
    if expected is None:
        assert result['exclusions']=={'nonconcrete_or_unsupported_definition':1}
        assert result['detection_recall'] is None


@pytest.mark.parametrize('mode',['recover','persistent','ordinary_timeout','drift'])
def test_recovery_once_preserves_deadline_inventory_and_configuration(monkeypatch,mode):
    from tier_a.oracles import LspOracle
    from tier_a import lsp_client
    o=LspOracle(['fake'],'.','rust',cargo_features=['mcp'])
    fd=FunctionDef('f','function',None,Location('a.rs',1,2),1)
    retained=time.monotonic()+1
    o.client.deadline=retained;o.client.stop=lambda:None
    old=o.client;created=[]
    class Client:
        def __init__(self,*a,**kw):
            created.append(kw);self.deadline=None
        def stop(self):pass
    monkeypatch.setattr(lsp_client,'LspClient',Client)
    o.start=lambda:None;o.wait_ready=lambda:None
    o.capability_probe=lambda:False
    o.document_symbols=lambda file:[] if mode=='drift' else [fd]
    def query(seed):
        if o.client is old or mode=='persistent':
            if mode!='ordinary_timeout':o.retries.append({'code':-32801})
            raise OracleTimeout('query deadline exhausted')
        return []
    o.callers=query;o.callees=lambda fd:[]
    if mode=='recover':
        assert o.prime_hierarchy([fd],['a.rs'],[fd])=={(fd,'callers'):[],(fd,'callees'):[]}
        assert o.client.deadline==retained
        assert created[0]['initialization_options']=={'cargo':{'features':['mcp']}}
        assert 0<created[0]['session_timeout']<=1
    else:
        with pytest.raises(OracleError):o.prime_hierarchy([fd],['a.rs'],[fd])
    assert len(created)==(0 if mode=='ordinary_timeout' else 1)
    assert len(o.restarts)==len(created)


@pytest.mark.parametrize('adapter',['lsp','tsserver'])
def test_missing_oracle_is_not_missing_input(adapter):
    from tier_a.cli import invalid_corpus_run
    from tier_a.oracles import LspOracle
    from tier_a.tsserver import TsserverOracle
    o=(LspOracle if adapter=='lsp' else TsserverOracle)(['/nonexistent/r1-oracle'],'.','rust' if adapter=='lsp' else 'ts')
    try:
        with pytest.raises(OracleError) as caught:o.start()
        result=invalid_corpus_run('x',{'path':'.','oracle':adapter},{},NS(date='test'),caught.value)
        assert result['meta']['invalid_reasons']==['oracle_unavailable']
        assert result['meta']['oracle_error_rate']==1 and result['meta']['sut_error_rate']==0
    finally:o.stop()


def test_same_line_seeds_are_retained_and_unaddressable(monkeypatch,tmp_path):
    seeds=[FunctionDef('same','method',container,Location('src/lib.rs',1,1),1,col) for container,col in [('A',3),('B',13)]]
    run=_runner(monkeypatch,tmp_path,seeds_override=seeds)
    probes=[p for key,p in run['probes'].items() if not key.startswith('_')]
    assert len(probes)==4  # both directions of both symbols, no overwrite
    assert {p['outcome'] for p in probes}=={'seed_unaddressable'}
    assert run['meta']['baseline_invalid'] and 'seed_unaddressable' in run['meta']['invalid_reasons']
    assert run['m1']['shared_oracle_selection_lines']==1


def test_sample_override_is_measured_and_clamps_population(monkeypatch,tmp_path):
    run=_runner(monkeypatch,tmp_path,default_per=8)
    assert not run['meta']['baseline_invalid']
    assert len([key for key in run['probes'] if not key.startswith('_')])==32
    assert run['probes']['_strata']['U-free']['population_symbols']==20


def test_unique_names_on_one_line_use_symbol_seeds(monkeypatch,tmp_path):
    seeds=[FunctionDef(name,'function',None,Location('src/lib.rs',1,1),1,col) for name,col in [('a',3),('b',13)]]
    run=_runner(monkeypatch,tmp_path,seeds_override=seeds)
    probes=[p for key,p in run['probes'].items() if not key.startswith('_')]
    assert len(probes)==4 and not run['meta']['baseline_invalid']
    assert {p['seed_address_mode'] for p in probes}=={'symbol'}


def test_native_sut_symbol_seeds_distinguish_same_line_targets(tmp_path):
    from tier_a.sut import PrismCli
    (tmp_path/'a.js').write_text('function alpha() {} function beta() {}\nfunction useA(){alpha();}\nfunction useB(){beta();}\n')
    sut=PrismCli.__new__(PrismCli);sut.bin=str(Path(__file__).parents[2]/'target/release/prism')
    sut.no_cache=True;sut.query_timeout_s=5
    defs=sut.inventory(str(tmp_path));alpha=next(f for f in defs if f.name=='alpha');beta=next(f for f in defs if f.name=='beta')
    sut.symbol_seeds={alpha,beta}
    assert [e.call_site.start_line for e in sut.callers(str(tmp_path),alpha)]==[2]
    assert [e.call_site.start_line for e in sut.callers(str(tmp_path),beta)]==[3]
    sut.unaddressable_seeds={alpha}
    with pytest.raises(SutError,match='unaddressable'):sut.callers(str(tmp_path),alpha)


def test_spotcheck_keeps_distinct_same_line_seed_identities(tmp_path):
    from collections import Counter
    from tier_a.cli import seed_id,run_m3_spotcheck
    seeds=[FunctionDef(name,'function',None,Location('a.ts',1,1),1,col) for name,col in [('a',9),('b',24)]]
    (tmp_path/'a.ts').write_text('function a(){} function b(){}\na(); b();\n')
    counts=Counter((fd.location.file,fd.selection_line) for fd in seeds)
    probes={}
    for fd in seeds:
        key=seed_id(fd,counts)
        probes['callers:'+key]={'outcome':'ok','direction':'callers','seed_def':key,'prism_sites':[['a.ts',2,2]],'oracle_sites':[]}
    o=NS(definitions_at=lambda inv,file,line,char:[DefTarget(Location('a.ts',1,1),'a' if char==0 else 'b','function')])
    result=run_m3_spotcheck(probes,o,seeds,str(tmp_path),10)
    assert len(result['checked'])==2 and result['counts']['confirmed_tp']==2


@pytest.mark.parametrize('sample',['0','-1'])
def test_sample_rejects_nonpositive_values(monkeypatch,capsys,sample):
    from tier_a.cli import main
    monkeypatch.setattr(sys,'argv',['tier-a','--sample',sample])
    with pytest.raises(SystemExit):main()
    assert 'sample must be positive' in capsys.readouterr().err


def test_native_sut_interior_seed_avoids_inline_default_callback(tmp_path):
    from tier_a.sut import PrismCli,configure_seed_addresses
    (tmp_path/'a.js').write_text('class A{deploy(){other();}}\nfunction target(){}\nfunction other(){}\nfunction deploy(x=function(){}){\n target();\n}\n')
    sut=PrismCli.__new__(PrismCli);sut.bin=str(Path(__file__).parents[2]/'target/release/prism')
    sut.no_cache=True;sut.query_timeout_s=5
    defs=sut.inventory(str(tmp_path));fd=next(f for f in defs if f.name=='deploy' and f.location.start_line==4)
    configure_seed_addresses(sut,defs)
    assert sut.location_seeds[fd]==5 and fd not in sut.unaddressable_seeds
    assert [e.call_site.start_line for e in sut.callees(str(tmp_path),fd)]==[5]
    a=FunctionDef('same','method',None,Location('x.js',1,1),1)
    configure_seed_addresses(sut,[a,a])
    assert a in sut.unaddressable_seeds


def test_nested_dependencies_change_manifest_validity(tmp_path):
    from tier_a.corpus import verify_source_manifest
    source=tmp_path/'source';source.mkdir();(source/'a.js').write_text('function a(){}')
    manifest=tmp_path/'manifest';manifest.write_text(hashlib.sha256((source/'a.js').read_bytes()).hexdigest()+'  a.js\n')
    cfg={'path':str(source),'source_manifest':str(manifest),'source_manifest_sha256':hashlib.sha256(manifest.read_bytes()).hexdigest(),'reject_node_modules':True}
    assert verify_source_manifest(cfg)[1]==[]
    (source/'deep'/'node_modules').mkdir(parents=True)
    assert 'oracle_node_modules_present:deep/node_modules' in verify_source_manifest(cfg)[1]


def test_m3_query_uses_original_utf16_position(tmp_path):
    from tier_a.cli import run_m3_spotcheck
    line='const q="😀abcdefghijkl"; target();'
    (tmp_path/'a.ts').write_text('function target(){}\n'+line+'\n')
    fd=FunctionDef('target','function',None,Location('a.ts',1,1),1)
    columns=[];expected=len(line[:line.index('target')].encode('utf-16-le'))//2
    def definitions(inv,file,number,column):
        columns.append(column)
        return [DefTarget(fd.location,'target','function')] if column==expected else []
    probes={'callers:a.ts:1':{'outcome':'ok','direction':'callers','seed_def':'a.ts:1',
                             'prism_sites':[['a.ts',2,2]],'oracle_sites':[]}}
    result=run_m3_spotcheck(probes,NS(definitions_at=definitions),[fd],str(tmp_path),10)
    assert columns==[expected] and result['counts']['confirmed_tp']==1
