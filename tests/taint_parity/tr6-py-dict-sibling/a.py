import os

def run(p):
    os.system(p['safe']) # @SINK

def main():
    t = acquire() # @SRC
    run({'cmd': t, 'safe': 'fixed'})
main()
