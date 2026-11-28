export function Html2(props: { html: string }) {
  const h = props.html;
  return <div dangerouslySetInnerHTML={{ __html: h }} />;
}
