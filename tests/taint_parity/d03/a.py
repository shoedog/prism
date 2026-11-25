import os

class Base:
    def run(self, cmd):
        os.system(cmd)  # @SINK

class Sub(Base):
    def run(self, cmd):
        print(cmd)

def main(r: Base):
    t = input()  # @SRC
    r.run(t)
