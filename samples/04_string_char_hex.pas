program s04;
type
  s9 = array[9] of char;
var
  s: s9;
begin
  s := 'ABC';
  Write(s);
  WriteLn;
  WriteLn(s[0]);
  WriteLn(s[1]);
  WriteLn(s[2]);
  WriteLn(s[3] = #0);

  s[0] := 'Z';
  Write(s);
  WriteLn;

  WriteLn(Ord('A'));
  WriteLn(Chr(66));
  IntToHex($2A, s, 8, true);
  WriteLn(s)
end.
