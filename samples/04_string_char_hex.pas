program s04;
type
  s8 = array[8] of char;
var
  s: s8;
begin
  s := 'ABC';
  WriteStr(s);
  WriteLn;
  WriteLn(s[0]);
  WriteLn(s[1]);
  WriteLn(s[2]);
  WriteLn(s[3] = #0);

  s[0] := 'Z';
  WriteStr(s);
  WriteLn;

  WriteLn(Ord('A'));
  WriteLn(Chr(66));
  WriteHex($2A);
  WriteLn
end.
