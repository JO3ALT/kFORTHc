program s21;
type
  arr = array[2] of integer;
var
  p: integer;
  a: arr;
  b: arr;
begin
  p := 7;
  a[0] := 1;
  a[1] := 2;
  b[0] := 100;
  b[1] := 200;

  a[2] := 999;
  a[-1] := 555;

  WriteLn(p);
  WriteLn(a[0]);
  WriteLn(a[1]);
  WriteLn(b[0]);
  WriteLn(b[1])
end.
