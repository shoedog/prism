const stage = "/blue-stage";
function gatewayPath(path) {
  return path.startsWith(stage + "/") ? path.slice(stage.length) : null;
}
