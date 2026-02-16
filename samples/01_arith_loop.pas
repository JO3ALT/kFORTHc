program s01;
var
  i: integer;
  s: integer;
begin
  s := 0;
  for i := 1 to 5 do
    s := s + i;
  WriteLn(s);

  i := 3;
  while i > 0 do
    begin
      Write(i);
      i := i - 1
    end;
  WriteLn;

  repeat
    s := s - 4
  until s <= 3;
  WriteLn(s);
  WriteLn(s = 3);
  WriteLn(2 * 5);
  WriteLn(7 - 4)
end.
