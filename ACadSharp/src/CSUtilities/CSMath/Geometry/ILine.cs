namespace CSMath.Geometry
{
	public interface ILine<T>
		where T : IVector
	{
		/// <summary>
		/// Origin point that the line intersects with
		/// </summary>
		public T Origin { get; set; }

		/// <summary>
		/// Direction fo the line
		/// </summary>
		public T Direction { get; set; }

		/// <summary>
		/// Find the intersection between 2 lines.
		/// </summary>
		/// <param name="line"></param>
		/// <returns>NaN if there is no intersection.</returns>
		T FindIntersection(ILine<T> line);
	}
}