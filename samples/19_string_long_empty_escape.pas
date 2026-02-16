program s19;
type
  s5 = array[5] of char;
  s8 = array[8] of char;
var
  a: s5;
  b: s8;
begin
  WriteLn('');
  WriteLn('A''B');

  a := 'ABCDEZ';
  WriteStr(a);
  WriteLn;
  WriteLn(a[4] = #0);

  a := '';
  WriteLn(a[0] = #0);

  b := 'HELLO';
  WriteStr(b);
  WriteLn;
  WriteLn(b[5] = #0)
end.
