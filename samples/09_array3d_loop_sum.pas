program s09;
type
  cube = array[2,3,4] of integer;
var
  c: cube;
  i: integer;
  j: integer;
  k: integer;
  sum: integer;
begin
  sum := 0;
  for i := 0 to 1 do
    for j := 0 to 2 do
      for k := 0 to 3 do
        begin
          c[i,j,k] := i * 100 + j * 10 + k;
          sum := sum + c[i,j,k]
        end;

  WriteLn(c[1,2,3]);
  WriteLn(sum);
  WriteLn(Length(c));
  WriteLn(Low(c));
  WriteLn(High(c))
end.
