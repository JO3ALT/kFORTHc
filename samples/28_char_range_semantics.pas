program s28;
var
  c: char;
begin
  c := Chr(256);
  WriteLn(Ord(c));

  c := Chr(-1);
  WriteLn(Ord(c));

  c := #255;
  WriteLn(Ord(c));

  WriteLn(Ord(Chr(0)));
  WriteLn(Ord(Chr(65)))
end.
