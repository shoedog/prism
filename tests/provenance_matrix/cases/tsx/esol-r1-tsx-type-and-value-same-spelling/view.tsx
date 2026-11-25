type Label = { text: string };

export function View(Label: Label) {
  return <span>{(Label as Label).text}</span>;
}
