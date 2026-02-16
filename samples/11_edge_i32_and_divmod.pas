program s11;
var
  x: integer;
  y: integer;
begin
  x := 2147483647;
  x := x + 1;
  WriteLn(x);

  y := -2147483647;
  y := y - 1;
  y := y - 1;
  WriteLn(y);

  WriteLn(-17 div 5);
  WriteLn(-17 mod 5);
  WriteLn(17 div -5);
  WriteLn(17 mod -5);

  if x < 0 then
    WriteLn(y > 0)
  else
    WriteLn(y < 0)
end.
