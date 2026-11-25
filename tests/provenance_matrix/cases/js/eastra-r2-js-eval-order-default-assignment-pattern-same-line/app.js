function labels() {
  let edition = process.env.EDITION;
  const values = [edition, ([edition = 'preview'] = []), edition];
  const final = edition;
  return final;
}
