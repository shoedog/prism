'use strict';
// E13 covers only JS/JSX files with a TS8xxx syntactic diagnostic.
function bucket(file,diagnostics=[]){
 if(!/\.(js|jsx)$/.test(file))return '3';
 if(diagnostics.some(d=>d.code>=8000&&d.code<=8999))return '1';
 return diagnostics.length?'2':'3';
}
function rowBucket(row,files){
 const bs=[row.from.file,row.to.file].map(f=>files.get(f)||'3');
 return bs.includes('1')?'1':bs.includes('2')?'2':'3';
}
function afterOverride(raw,proof){return proof?.status==='EARLY_ERROR'?'WRONG':raw;}
module.exports={bucket,rowBucket,afterOverride};
