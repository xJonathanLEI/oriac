%builtins output

func main(output_ptr) -> (output_ptr):
    [ap] = 0; ap++
    [ap - 1] = [output_ptr]
    [ap] = output_ptr + 3; ap++  # The correct return value is output_ptr + 1
    ret
end
