import paramiko
def f(inp):
    t = inp.value # @SRC
    paramiko.SSHClient().exec_command(t) # @SINK
