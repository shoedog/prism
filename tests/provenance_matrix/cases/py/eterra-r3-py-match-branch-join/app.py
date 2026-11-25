import os
def run(choice):
    value = 'default'
    match choice:
        case 1:
            value = os.getenv('MODE')
        case 2:
            value = 'other'
    return value
