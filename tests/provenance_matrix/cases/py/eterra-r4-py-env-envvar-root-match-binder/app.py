import os
def run(subject):
    match subject:
        case {'os': os}:
            mode = os.getenv('MODE')
            print(mode)
