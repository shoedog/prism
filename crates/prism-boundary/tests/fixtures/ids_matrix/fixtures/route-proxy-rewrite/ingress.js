const ingress = { publicPrefix: "/public/pottery", upstreamPrefix: "" };
function upstreamPath(path) {
  return path.startsWith(ingress.publicPrefix + "/")
    ? ingress.upstreamPrefix + path.slice(ingress.publicPrefix.length)
    : null;
}
