const edition = process.env.EDITION;
export function Card(
  caption = edition,
) {
  var edition = 'preview';
  return <span>{edition}</span>;
}
