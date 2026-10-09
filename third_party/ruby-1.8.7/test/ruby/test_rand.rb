require 'test/unit'

class TestRand < Test::Unit::TestCase
  # vkxp: srand and rand follow Ruby 1.8.1 (see PATCHES.md), so the sequences pinned here are
  # the 1.8.1 ones wherever the limit is a Fixnum. test_mt now only shows that a seed counts
  # by its low 32 bits. The two tests with Bignum limits keep their upstream values.
  def test_mt
    srand(0x00000456_00000345_00000234_00000123)
    %w(422279212 2896376844 2794392594 1785336940 3256458092).each {|w|
      assert_equal(w.to_i, rand(0x100000000))
    }
  end

  def test_0x3fffffff
    srand(0)
    %w(589284012 767928734 647212246 585063862 454895876).each {|w|
      assert_equal(w.to_i, rand(0x3fffffff))
    }
  end

  def test_0x40000000
    srand(0)
    %w(589284012 767928734 647212246 585063862 454895876).each {|w|
      assert_equal(w.to_i, rand(0x40000000))
    }
  end

  def test_0x40000001
    srand(0)
    %w(589284013 767928735 647212247 585063863 454895877).each {|w|
      assert_equal(w.to_i, rand(0x40000001))
    }
  end

  def test_0xffffffff
    srand(0)
    %w(2357136050 3071714938 2588848986 2340255450 1819583507).each {|w|
      assert_equal(w.to_i, rand(0xffffffff))
    }
  end

  def test_0x100000000
    srand(0)
    %w(2357136050 3071714939 2588848987 2340255451 1819583507).each {|w|
      assert_equal(w.to_i, rand(0x100000000))
    }
  end

  def test_0x100000001
    srand(0)
    %w(2357136051 3071714939 2588848988 2340255451 1819583508).each {|w|
      assert_equal(w.to_i, rand(0x100000001))
    }
  end

  def test_rand_0x100000000
    srand(311702798)
    %w(3888582751 3778706670 1249045523 3287113490 882162109).each {|w|
      assert_equal(w.to_i, rand(0x100000001))
    }
  end

  def test_0x1000000000000
    srand(0)
    %w(154477268236437
       201307910243385
       169662807241802
       153370981244079
       119248224777275).each {|w|
      assert_equal(w.to_i, rand(0x1000000000000))
    }
  end

  def test_0x1000000000001
    srand(0)
    %w(154477268236437
       201307910243386
       169662807241802
       153370981244080
       119248224777276).each {|w|
      assert_equal(w.to_i, rand(0x1000000000001))
    }
  end

  def test_0x3fffffffffffffff
    srand(0)
    %w(2530955562785787904
       3298228801427629568
       2779755433849689600
       2512830156703001088
       1953762914750887424).each {|w|
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
    ws = %w(605608698435 778616765152 319853143009 561660891703 981805559292)
    srand(3)
    ws.each {|w| assert_equal(w.to_i, rand(0x10000000000)) }
    srand(3)
    ws.each {|w| assert_equal(w.to_i, rand(-0x10000000000)) }
  end

  def test_neg_0x10000
    ws = %w(35967 46870 39502 35709 27764)
    srand(0)
    ws.each {|w| assert_equal(w.to_i, rand(0x10000)) }
    srand(0)
    ws.each {|w| assert_equal(w.to_i, rand(-0x10000)) }
  end

end
