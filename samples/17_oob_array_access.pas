program s17;
type
  arr = array[2] of integer;
var
  a: arr;
begin
  a[0] := 10;
  a[1] := 20;
  a[2] := 30;
  a[-1] := 40;
  WriteLn(a[0]);
  WriteLn(a[1]);
  WriteLn(a[2]);
  WriteLn(a[-1])
end.
