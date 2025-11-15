from flask import Flask

app = Flask(__name__)


@app.route("/")
@app.route("/hello")
@app.route("/<path:path>")
def hello(path=None):
    return "Hello, RS144!"


if __name__ == "__main__":
    app.run(host="127.0.0.1", port=8080, debug=True)
