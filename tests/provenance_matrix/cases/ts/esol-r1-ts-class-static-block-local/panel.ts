export class Panel {
  static caption = 'fixed';

  static {
    const mode = process.env.MODE;
    Panel.caption = mode;
  }
}
