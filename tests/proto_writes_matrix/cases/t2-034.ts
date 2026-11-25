const Bucket = { Records: "records" } as const;
function put(root: any, property: string, value: unknown) {
  root[Bucket.Records][property] = value;
  return root;
}

module.exports = put;
