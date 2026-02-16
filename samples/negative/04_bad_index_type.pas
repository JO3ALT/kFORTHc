program n04;
type
  arr = array[2] of integer;
var
  a: arr;
  b: boolean;
begin
  b := 1 = 1;
  a[b] := 1
end.
