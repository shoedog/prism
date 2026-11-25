const title = process.env.TITLE;
declare function show(node: unknown): void;
export class Card {
  constructor(readonly title: string) {
    show(<span title={title} />);
  }
  static preview() {
    return <span>{title}</span>;
  }
}
