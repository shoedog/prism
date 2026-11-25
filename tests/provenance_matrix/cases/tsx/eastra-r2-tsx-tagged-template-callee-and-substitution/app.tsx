const edition = process.env.EDITION;
type Formatter = (parts: TemplateStringsArray, value: string | undefined) => string;
export function View(format: Formatter) {
  return <span title={format`format ${edition}`} />;
}
