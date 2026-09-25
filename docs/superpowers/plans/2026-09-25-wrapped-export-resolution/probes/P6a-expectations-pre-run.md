# P6a prototype control expectations (written BEFORE running)
Proto binary sha256 b902b190...; diff = ../proto-diff.txt
C02 Exact import_member -> lib.tsx:Island@2-4 | C08 drop | C09 drop | C10 drop (F4 hazard refused)
C11 Exact -> Island@6-8 ONLY (not nested @3-3) | C12 drop (impostor) | C13 drop | C14 Exact via chain
C15 Exact -> Island (named fe) | C16 drop | C17 A Exact, B Exact | C18 not import_member | C19 drop (observer not admitted)
C20 drop (let) | C21 unchanged Exact | C22 drop | C23 drop | C24 drop | C25 drop (cast)
C01, C03-C07 unchanged vs baseline
