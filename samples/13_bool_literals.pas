program s13;
var
  t: boolean;
  f: boolean;
  x: integer;
begin
  t := true;
  f := false;
  if t then
    WriteLn('T-VAR')
  else
    WriteLn('T-VAR-FAIL');
  if f then
    WriteLn('F-VAR-FAIL')
  else
    WriteLn('F-VAR');

  if true then
    WriteLn('T-IF')
  else
    WriteLn('T-ELSE');

  if false then
    WriteLn('F-IF')
  else
    WriteLn('F-ELSE');

  x := 0;
  if t then
    x := x + 10;
  if f then
    x := x + 100
  else
    x := x + 1;
  WriteLn(x)
end.
