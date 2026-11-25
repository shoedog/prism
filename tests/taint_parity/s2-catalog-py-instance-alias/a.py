from paramiko import SSHClient as Client

def entry():
    t = acquire() # @SRC
    session = Client()
    session.exec_command(t) # @SINK

entry()
