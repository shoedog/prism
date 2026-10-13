#!/usr/bin/env python3
"""Tier-A adapter: bind every prism invocation to the user-authorized E5 cache."""
import os,sys
binary=os.environ['E5_SUT_BIN'];cache=os.path.expanduser('~/prism-evidence/js-e5/cache');args=sys.argv[1:]
if args and args[0]=='nav':
 if '--no-cache' in args:args.remove('--no-cache')
 if '--cache-dir' in args:
  i=args.index('--cache-dir');args[i+1]=cache
 else:args[1:1]=['--cache-dir',cache]
else:args[0:0]=['--cache-dir',cache]
# The owner's cache binding supersedes the matrix runner's default no-cache.
# Report this adapter explicitly; do not claim the unmodified harness command.
os.execv(binary,[binary,*args])
