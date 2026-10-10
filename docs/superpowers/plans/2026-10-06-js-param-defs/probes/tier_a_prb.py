#!/usr/bin/env python3
"""Trusted Tier-A CLI using the owner-selected lane cache. No corpus execution."""
import sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[5]/'eval'))
from tier_a.sut import PrismCli
original=PrismCli.__init__
def init(self,*args,**kwargs):
 original(self,*args,**kwargs)
 self.cache_dir=str(Path.home()/'prism-evidence/js-param-defs/cache')
PrismCli.__init__=init
from tier_a.cli import main
if __name__=='__main__':raise SystemExit(main())
