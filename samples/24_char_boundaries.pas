program s24;
var
  c: char;
begin
  c := #0;
  WriteLn(Ord(c));

  c := #255;
  WriteLn(Ord(c));

  WriteLn(Ord(Chr(0)));
  WriteLn(Ord(Chr(255)));

  c := Chr(256);
  WriteLn(Ord(c))
end.
