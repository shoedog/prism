use prism::ast::ParsedFile;
use prism::languages::Language;
fn main(){
 let cases=[
 ("object", "function take({x}, later){return later;}"),
 ("alias", "function take(first, {key: value}, later){return later;}"),
 ("nested_default", "function take({x = init()}, later){return later;}"),
 ("array", "function take([x], later){return later;}"),
 ("rest", "function take(...items){return items;}"),
 ("duplicate", "function take({x}, x){return x;}"),
 ("assigned", "const take = ({x}, later) => later;"),
 ("wrapped", "const take = memo(({x}, later) => later);"),
 ("unnamed", "consume(({x}, later) => later);"),
 ("shadow", "function take({x}, later){function nested(later){return later;}return later;}"),
 ];
 for lang in [Language::JavaScript,Language::TypeScript,Language::Tsx]{for (id,src) in cases{
 let p=ParsedFile::parse("fixture",src,lang).unwrap();println!("CASE {id} {lang:?} errors={}",p.parse_error_count);
 for n in p.all_functions(){println!("FN {:?} {} {} slots={:?} occurrences={:?}",p.language.function_name(&n).map(|x|p.node_text(&x)),n.start_byte(),n.end_byte(),p.function_parameter_slot_occurrences(&n),p.function_parameter_occurrences(&n));}
 }}
}
