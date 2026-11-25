import { Component } from 'react';
export class Preview extends Component {
  readonly label = process.env.PREVIEW_LABEL;
  render() {
    const title = this.label;
    return <span>{title}</span>;
  }
}
