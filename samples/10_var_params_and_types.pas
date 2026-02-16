program s10;
type
  arr4 = array[4] of integer;
  pair = record
    a: integer;
    b: integer;
  end;
var
  a: arr4;
  r: pair;

procedure Fill(var x: arr4; base: integer);
var
  i: integer;
begin
  for i := 0 to 3 do
    x[i] := base + i
end;

procedure Bump(var z: pair; d: integer);
begin
  z.a := z.a + d;
  z.b := z.b + d * 2
end;

begin
  Fill(a, 5);
  r.a := 1;
  r.b := 2;
  Bump(r, 3);

  WriteLn(a[0]);
  WriteLn(a[3]);
  WriteLn(r.a);
  WriteLn(r.b);
  WriteLn(a[0] + a[1] + a[2] + a[3])
end.
