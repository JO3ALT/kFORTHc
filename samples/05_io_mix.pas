program s05;
type
  iarr = array[4] of integer;
  s8 = array[8] of char;
var
  i: integer;
  b: boolean;
  c: char;
  a: iarr;
  s: s8;
begin
  Read(i);
  ReadLn;
  Read(b, c);
  ReadArr(a, 3);
  ReadStr(s, 5);

  WriteLn(i);
  WriteLn(b);
  WriteLn(c);
  WriteArr(a, 3);
  WriteStr(s);
  WriteLn
end.
