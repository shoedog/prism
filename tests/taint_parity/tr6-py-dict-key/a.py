import os

def run(p):
    os.system(p['cmd']) # @SINK

def main():
    t = acquire() # @SRC
    run({'cmd': t, 'other': 'fixed'})
main()
