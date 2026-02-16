program s14;
var
  i: integer;
  x: integer;
begin
  x := 0;
  for i := 5 downto 1 do
    x := x + i;
  WriteLn(x);

  for i := 3 downto 1 do
    Write(i);
  WriteLn;

  case x of
    15: WriteLn('HIT-15')
  end;

  case i of
    1: WriteLn('HIT-1');
    2: WriteLn('HIT-2')
  else
    WriteLn('MISS')
  end
end.
