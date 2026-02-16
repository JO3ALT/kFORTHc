program s23;

function Fib(n: integer): integer;
begin
  if n <= 1 then
    Fib := n
  else
    Fib := Fib(n - 1) + Fib(n - 2)
end;

begin
  WriteLn(Fib(6))
end.
