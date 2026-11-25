from flask import Flask

app = Flask(__name__, static_folder=None)


def preview(_view):
    def replacement():
        return 'preview only'

    return replacement


@app.get('/draft')
@preview
@app.post('/draft')
def submit():
    return 'saved'
