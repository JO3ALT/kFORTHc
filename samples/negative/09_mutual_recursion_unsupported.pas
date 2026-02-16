program n09;

function A(n: integer): integer;
begin
  if n = 0 then
    A := 0
  else
    A := B(n - 1)
end;

function B(n: integer): integer;
begin
  B := A(n)
end;

begin
  WriteLn(A(3))
end.
