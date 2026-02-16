program s25;
var
  x: integer;
  y: integer;
  z: integer;
begin
  x := 2147483647;
  y := x + 1;
  WriteLn(y);
  WriteLn(y < 0);

  z := -2147483647;
  z := z - 2;
  WriteLn(z);
  WriteLn(z > 0);

  y := 50000 * 50000;
  WriteLn(y);
  WriteLn(y < 0)
end.
