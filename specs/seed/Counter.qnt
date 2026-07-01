module Counter {
  var x: int

  action init = x' = 0
  action step = x' = x + 1

  val not_two = x != 2
}
