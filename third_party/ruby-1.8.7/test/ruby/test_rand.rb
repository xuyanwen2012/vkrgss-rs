require 'test/unit'

class TestRand < Test::Unit::TestCase
  # vkxp: srand and rand follow Ruby 1.8.1 (see PATCHES.md), so the sequences pinned here are
  # the 1.8.1 ones wherever the limit is a Fixnum: one 32-bit output divided by 2^32, scaled
  # and truncated. test_mt now only shows that a seed counts by its low 32 bits. The two tests
  # with Bignum limits keep their upstream values.
  def test_mt
    srand(0x00000456_00000345_00000234_00000123)
    %w(422279215 1698001409 2896376837 1703936274 2794392607).each {|w|
      assert_equal(w.to_i, rand(0x100000000))
    }
  end

  def test_0x3fffffff
    srand(0)
    %w(589284010 636562059 767928732 906523439 647212240).each {|w|
      assert_equal(w.to_i, rand(0x3fffffff))
    }
  end

  def test_0x40000000
    srand(0)
    %w(589284011 636562059 767928733 906523440 647212240).each {|w|
      assert_equal(w.to_i, rand(0x40000000))
    }
  end

  def test_0x40000001
    srand(0)
    %w(589284011 636562060 767928733 906523440 647212241).each {|w|
      assert_equal(w.to_i, rand(0x40000001))
    }
  end

  def test_0xffffffff
    srand(0)
    %w(2357136043 2546248238 3071714932 3626093759 2588848962).each {|w|
      assert_equal(w.to_i, rand(0xffffffff))
    }
  end

  def test_0x100000000
    srand(0)
    %w(2357136044 2546248239 3071714933 3626093760 2588848963).each {|w|
      assert_equal(w.to_i, rand(0x100000000))
    }
  end

  def test_0x100000001
    srand(0)
    %w(2357136044 2546248239 3071714933 3626093760 2588848963).each {|w|
      assert_equal(w.to_i, rand(0x100000001))
    }
  end

  def test_rand_0x100000000
    srand(311702798)
    %w(3888582720 4119812344 3778706665 1867155870 1249045521).each {|w|
      assert_equal(w.to_i, rand(0x100000001))
    }
  end

  def test_0x1000000000000
    srand(0)
    %w(154477267779584
       166870924591104
       201307909849088
       237639680655360
       169662805639168).each {|w|
      assert_equal(w.to_i, rand(0x1000000000000))
    }
  end

  def test_0x1000000000001
    srand(0)
    %w(154477267779584
       166870924591104
       201307909849088
       237639680655360
       169662805639168).each {|w|
      assert_equal(w.to_i, rand(0x1000000000001))
    }
  end

  def test_0x3fffffffffffffff
    srand(0)
    %w(2530955555300704256
       2734013228500647936
       3298228794967457792
       3893488527857418240
       2779755407592128512).each {|w|
      assert_equal(w.to_i, rand(0x3fffffffffffffff))
    }
  end

  def test_0x4000000000000000
    srand(0)
    %w(900450186894289455
       3969543146641149120
       1895649597198586619
       827948490035658087
       3203365596207111891).each {|w|
      assert_equal(w.to_i, rand(0x4000000000000000))
    }
  end

  def test_0x4000000000000001
    srand(0)
    %w(900450186894289455
       3969543146641149120
       1895649597198586619
       827948490035658087
       2279347887019741461).each {|w|
      assert_equal(w.to_i, rand(0x4000000000000001))
    }
  end

  def test_neg_0x10000000000
    ws = %w(605608700416 77762828288 778616764672 923533738752 319853148160)
    srand(3)
    ws.each {|w| assert_equal(w.to_i, rand(0x10000000000)) }
    srand(3)
    ws.each {|w| assert_equal(w.to_i, rand(-0x10000000000)) }
  end

  def test_neg_0x10000
    ws = %w(35967 38852 46870 55329 39502)
    srand(0)
    ws.each {|w| assert_equal(w.to_i, rand(0x10000)) }
    srand(0)
    ws.each {|w| assert_equal(w.to_i, rand(-0x10000)) }
  end

end
